use rarag_core::daemon::{DaemonRequest, DaemonResponse, ErrorResponse};
use rarag_core::ipc::{DAEMON_READ_TIMEOUT, decode_frame_len, encode_framed_message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

pub async fn read_request(stream: &mut UnixStream) -> Result<DaemonRequest, String> {
    let mut header = [0_u8; 4];
    timeout(DAEMON_READ_TIMEOUT, stream.read_exact(&mut header))
        .await
        .map_err(|_| "daemon request timed out".to_string())?
        .map_err(|err| err.to_string())?;
    let len = decode_frame_len(header)?;
    let mut body = vec![0_u8; len];
    timeout(DAEMON_READ_TIMEOUT, stream.read_exact(&mut body))
        .await
        .map_err(|_| "daemon request timed out".to_string())?
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map_err(|err| err.to_string())
}

pub async fn write_response(
    stream: &mut UnixStream,
    response: &DaemonResponse,
) -> Result<(), String> {
    let body = serde_json::to_vec(response).map_err(|err| err.to_string())?;
    let framed = encode_framed_message(&body)?;
    stream
        .write_all(&framed)
        .await
        .map_err(|err| err.to_string())?;
    stream.shutdown().await.map_err(|err| err.to_string())
}

pub fn error_response(message: impl Into<String>) -> DaemonResponse {
    DaemonResponse::Error(ErrorResponse {
        message: message.into(),
    })
}
