#[derive(Debug, thiserror::Error)]
pub enum RealtimeError {
    #[error("HTTP request failed")]
    Http(#[from] reqwest::Error),
    #[error("failed to decode protobuf response")]
    Decode(#[from] prost::DecodeError),
}
