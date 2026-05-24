use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionEvent {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoSwitch {
    pub group: String,
    pub from: String,
    pub to: String,
    pub from_country: String,
    pub to_country: String,
    pub at: i64,
}
