use rarag_core::daemon::{DaemonRequest, DaemonResponse, ErrorResponse};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub async fn read_request(stream: &mut UnixStream) -> Result<DaemonRequest, String> {
    let mut body = Vec::new();
    stream
        .read_to_end(&mut body)
        .await
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map_err(|err| err.to_string())
}

pub async fn write_response(
    stream: &mut UnixStream,
    response: &DaemonResponse,
) -> Result<(), String> {
    let body = serde_json::to_vec(response).map_err(|err| err.to_string())?;
    stream.write_all(&body).await.map_err(|err| err.to_string())?;
    stream.shutdown().await.map_err(|err| err.to_string())
}

pub fn error_response(message: impl Into<String>) -> DaemonResponse {
    DaemonResponse::Error(ErrorResponse {
        message: message.into(),
    })
}
