#[derive(Debug, thiserror::Error)]
pub enum ClashError {
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("auth required or wrong secret")]
    Auth,
    #[error("unexpected status {0}")]
    BadStatus(u16),
    #[error("decode error: {0}")]
    Decode(String),
}
