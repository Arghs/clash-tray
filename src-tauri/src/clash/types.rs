use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct ProxiesResponse {
    pub proxies: HashMap<String, Proxy>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Proxy {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Vec<String>,
    #[serde(default)]
    pub history: Vec<DelayHistory>,
    #[serde(default)]
    pub udp: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DelayHistory {
    pub time: String,
    pub delay: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionInfo {
    pub version: String,
    #[serde(default)]
    pub premium: bool,
    #[serde(default)]
    pub meta: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionsResponse {
    #[serde(rename = "downloadTotal", default)]
    pub download_total: u64,
    #[serde(rename = "uploadTotal", default)]
    pub upload_total: u64,
    /// Mihomo's RAM use in bytes (some builds). Optional.
    #[serde(default)]
    pub memory: Option<u64>,
    /// Skip-deserializing connection payloads; we only care about the count.
    #[serde(default)]
    pub connections: Vec<serde_json::Value>,
}
