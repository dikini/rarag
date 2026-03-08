use std::io::{Read, Write};
use std::time::Duration;

pub const DAEMON_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const DAEMON_READ_TIMEOUT: Duration = Duration::from_secs(1);

pub fn encode_framed_message(body: &[u8]) -> Result<Vec<u8>, String> {
    let len = u32::try_from(body.len()).map_err(|_| "daemon message too large".to_string())?;
    let mut framed = Vec::with_capacity(4 + body.len());
    framed.extend_from_slice(&len.to_be_bytes());
    framed.extend_from_slice(body);
    Ok(framed)
}

pub fn decode_frame_len(header: [u8; 4]) -> Result<usize, String> {
    let len = u32::from_be_bytes(header) as usize;
    if len > DAEMON_MAX_MESSAGE_BYTES {
        return Err(format!(
            "daemon message too large: {len} bytes exceeds limit {DAEMON_MAX_MESSAGE_BYTES}"
        ));
    }
    Ok(len)
}

pub fn write_framed_message<W: Write>(writer: &mut W, body: &[u8]) -> Result<(), String> {
    let framed = encode_framed_message(body)?;
    writer.write_all(&framed).map_err(|err| err.to_string())
}

pub fn read_framed_message<R: Read>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut header = [0_u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|err| err.to_string())?;
    let len = decode_frame_len(header)?;
    let mut body = vec![0_u8; len];
    reader
        .read_exact(&mut body)
        .map_err(|err| err.to_string())?;
    Ok(body)
}
