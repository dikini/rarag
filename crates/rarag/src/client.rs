use std::os::unix::net::UnixStream;
use std::path::Path;

use rarag_core::daemon::{DaemonRequest, DaemonResponse, ErrorResponse};
use rarag_core::ipc::{read_framed_message, write_framed_message};

pub fn send_request(socket_path: &Path, request: &DaemonRequest) -> Result<DaemonResponse, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|err| err.to_string())?;
    let body = serde_json::to_vec(request).map_err(|err| err.to_string())?;
    write_framed_message(&mut stream, &body)?;
    let response = read_framed_message(&mut stream)?;
    let response: DaemonResponse =
        serde_json::from_slice(&response).map_err(|err| err.to_string())?;
    if let DaemonResponse::Error(ErrorResponse { message }) = &response {
        Err(message.clone())
    } else {
        Ok(response)
    }
}
