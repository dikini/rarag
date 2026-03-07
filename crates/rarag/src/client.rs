use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use rarag_core::daemon::{DaemonRequest, DaemonResponse, ErrorResponse};

pub fn send_request(socket_path: &Path, request: &DaemonRequest) -> Result<DaemonResponse, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|err| err.to_string())?;
    let body = serde_json::to_vec(request).map_err(|err| err.to_string())?;
    stream.write_all(&body).map_err(|err| err.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|err| err.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| err.to_string())?;
    let response: DaemonResponse =
        serde_json::from_slice(&response).map_err(|err| err.to_string())?;
    if let DaemonResponse::Error(ErrorResponse { message }) = &response {
        Err(message.clone())
    } else {
        Ok(response)
    }
}
