use std::collections::HashMap;
use std::time::Duration;

use reqwest::{Method, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::json;

use super::errors::ClashError;
use super::types::{ConnectionsResponse, ProxiesResponse, VersionInfo};

pub struct ClashClient {
    base: String,
    secret: Option<String>,
    http: reqwest::Client,
}

impl ClashClient {
    pub fn new(base: impl Into<String>, secret: Option<String>) -> Self {
        let base = base.into().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client builds with rustls feature");
        Self { base, secret, http }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn req(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base, path);
        let mut req = self.http.request(method, &url);
        if let Some(s) = &self.secret {
            req = req.bearer_auth(s);
        }
        req
    }

    pub async fn version(&self) -> Result<VersionInfo, ClashError> {
        let resp = self.req(Method::GET, "/version").send().await?;
        decode_json(check_status(resp)?).await
    }

    pub async fn proxies(&self) -> Result<ProxiesResponse, ClashError> {
        let resp = self.req(Method::GET, "/proxies").send().await?;
        decode_json(check_status(resp)?).await
    }

    pub async fn select(&self, group: &str, node: &str) -> Result<(), ClashError> {
        let resp = self
            .req(Method::PUT, &format!("/proxies/{group}"))
            .json(&json!({ "name": node }))
            .send()
            .await?;
        check_status(resp)?;
        Ok(())
    }

    pub async fn connections(&self) -> Result<ConnectionsResponse, ClashError> {
        let resp = self.req(Method::GET, "/connections").send().await?;
        decode_json(check_status(resp)?).await
    }

    pub async fn group_delay(&self, group: &str) -> Result<HashMap<String, u32>, ClashError> {
        let resp = self
            .req(
                Method::GET,
                &format!(
                    "/group/{group}/delay?url=http%3A%2F%2Fwww.gstatic.com%2Fgenerate_204&timeout=3000"
                ),
            )
            .send()
            .await?;
        decode_json(check_status(resp)?).await
    }
}

fn check_status(resp: Response) -> Result<Response, ClashError> {
    let status = resp.status();
    if status.is_success() {
        Ok(resp)
    } else if status == StatusCode::UNAUTHORIZED {
        Err(ClashError::Auth)
    } else {
        Err(ClashError::BadStatus(status.as_u16()))
    }
}

async fn decode_json<T: DeserializeOwned>(resp: Response) -> Result<T, ClashError> {
    resp.json::<T>().await.map_err(|e| {
        if e.is_decode() {
            ClashError::Decode(e.to_string())
        } else {
            ClashError::Network(e)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, header_exists, method as m, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn proxies_body() -> &'static str {
        r#"{"proxies":{
            "PROXY":{"name":"PROXY","type":"Selector","now":"HK-01","all":["HK-01"],"history":[],"udp":true},
            "HK-01":{"name":"HK-01","type":"Shadowsocks","history":[{"time":"2026-05-16T12:34:56Z","delay":18}],"udp":true}
        }}"#
    }

    #[tokio::test]
    async fn proxies_happy_path() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(200).set_body_string(proxies_body()))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        let p = c.proxies().await.unwrap();
        assert_eq!(p.proxies.len(), 2);
        assert_eq!(p.proxies["PROXY"].now.as_deref(), Some("HK-01"));
        assert_eq!(p.proxies["HK-01"].history.first().unwrap().delay, 18);
    }

    #[tokio::test]
    async fn unauthorized_maps_to_auth_error() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), Some("wrong".into()));
        let err = c.proxies().await.unwrap_err();
        assert!(matches!(err, ClashError::Auth), "got: {err:?}");
    }

    #[tokio::test]
    async fn server_error_maps_to_bad_status() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        let err = c.proxies().await.unwrap_err();
        assert!(matches!(err, ClashError::BadStatus(500)), "got: {err:?}");
    }

    #[tokio::test]
    async fn malformed_body_maps_to_decode_error() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(200).set_body_string("definitely not json"))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        let err = c.proxies().await.unwrap_err();
        assert!(matches!(err, ClashError::Decode(_)), "got: {err:?}");
    }

    #[tokio::test]
    async fn bearer_attached_when_secret_set() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .and(header("Authorization", "Bearer s3cret"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"proxies":{}}"#))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), Some("s3cret".into()));
        c.proxies().await.unwrap();
    }

    #[tokio::test]
    async fn no_authorization_header_when_secret_none() {
        let server = MockServer::start().await;
        // Only respond if Authorization is missing — any request carrying it returns 500.
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .and(header_exists("Authorization"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        Mock::given(m("GET"))
            .and(path("/proxies"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"proxies":{}}"#))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        c.proxies().await.unwrap();
    }

    #[tokio::test]
    async fn select_sends_name_body() {
        let server = MockServer::start().await;
        Mock::given(m("PUT"))
            .and(path("/proxies/PROXY"))
            .and(body_json(json!({"name": "HK-01"})))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        c.select("PROXY", "HK-01").await.unwrap();
    }

    #[tokio::test]
    async fn version_parses_meta_flag() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/version"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"version":"1.18.0","meta":true}"#),
            )
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        let v = c.version().await.unwrap();
        assert_eq!(v.version, "1.18.0");
        assert!(v.meta);
        assert!(!v.premium);
    }

    #[tokio::test]
    async fn group_delay_returns_map() {
        let server = MockServer::start().await;
        Mock::given(m("GET"))
            .and(path("/group/AUTO/delay"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"HK-01":18,"HK-02":23,"JP-01":67}"#),
            )
            .mount(&server)
            .await;
        let c = ClashClient::new(server.uri(), None);
        let d = c.group_delay("AUTO").await.unwrap();
        assert_eq!(d.get("HK-01"), Some(&18));
        assert_eq!(d.get("JP-01"), Some(&67));
    }
}
