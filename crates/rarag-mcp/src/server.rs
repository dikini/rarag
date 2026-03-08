use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rarag_core::daemon::{DaemonRequest, QueryPayload};
use rarag_core::ipc::{
    LOCAL_IPC_MAX_MESSAGE_BYTES, LOCAL_IPC_READ_TIMEOUT, read_framed_message, write_framed_message,
};
use rarag_core::retrieval::QueryMode;
use rarag_core::unix_socket::prepare_socket_path;
use serde_json::{Value, json};

use crate::tools::{McpRequest, McpResponse, tool_definitions};

pub fn serve(socket_path: &Path, daemon_socket: &Path) -> Result<(), String> {
    prepare_socket_path(socket_path)?;
    let listener = UnixListener::bind(socket_path).map_err(|err| err.to_string())?;

    for stream in listener.incoming() {
        let mut stream = stream.map_err(|err| err.to_string())?;
        let response = match read_request_value(&mut stream) {
            Ok(request) => handle_request_value(request, daemon_socket),
            Err(err) => json!({ "error": err }),
        };
        write_response_value(&mut stream, &response)?;
    }

    Ok(())
}

pub fn serve_stdio(daemon_socket: &Path) -> Result<(), String> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    while let Some(request) = read_stdio_request_value(&mut reader)? {
        let response = handle_request_value(request, daemon_socket);
        write_stdio_response_value(&mut writer, &response)?;
    }

    Ok(())
}

fn handle_request_value(request: Value, daemon_socket: &Path) -> Value {
    if is_jsonrpc_request(&request) {
        handle_jsonrpc_request(request, daemon_socket)
    } else {
        handle_legacy_request(request, daemon_socket)
    }
}

fn handle_jsonrpc_request(request: Value, daemon_socket: &Path) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "initialize" => jsonrpc_result(
            id,
            json!({
                "protocolVersion": params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("2025-03-26"),
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "rarag-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),
        "notifications/initialized" => jsonrpc_result(id, Value::Null),
        "tools/list" => jsonrpc_result(id, json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match map_tool_call(name, arguments) {
                Ok(request) => match send_daemon_request(daemon_socket, &request) {
                    Ok(response) => {
                        let payload =
                            serde_json::to_value(&response).expect("serialize daemon response");
                        if payload.get("kind").and_then(Value::as_str) == Some("error") {
                            jsonrpc_error(
                                id,
                                -32000,
                                payload["message"].as_str().unwrap_or("daemon error"),
                            )
                        } else {
                            jsonrpc_result(
                                id,
                                json!({
                                    "content": [{
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&payload)
                                            .expect("serialize structured content")
                                    }],
                                    "structuredContent": payload,
                                    "isError": false
                                }),
                            )
                        }
                    }
                    Err(err) => jsonrpc_error(id, -32000, &err),
                },
                Err(err) => jsonrpc_error(id, -32602, &err),
            }
        }
        _ => jsonrpc_error(id, -32601, &format!("unsupported method {method}")),
    }
}

fn handle_legacy_request(request: Value, daemon_socket: &Path) -> Value {
    match serde_json::from_value::<McpRequest>(request) {
        Ok(McpRequest::ListTools) => serde_json::to_value(McpResponse::Tools {
            tools: tool_definitions(),
        })
        .expect("serialize tools response"),
        Ok(McpRequest::CallTool { name, arguments }) => {
            let response = match map_tool_call(&name, arguments) {
                Ok(request) => match send_daemon_request(daemon_socket, &request) {
                    Ok(response) => McpResponse::CallResult {
                        result: serde_json::to_value(response).expect("serialize daemon response"),
                    },
                    Err(err) => McpResponse::Error { message: err },
                },
                Err(err) => McpResponse::Error { message: err },
            };
            serde_json::to_value(response).expect("serialize legacy response")
        }
        Err(err) => serde_json::to_value(McpResponse::Error {
            message: err.to_string(),
        })
        .expect("serialize legacy error"),
    }
}

fn is_jsonrpc_request(request: &Value) -> bool {
    request
        .get("jsonrpc")
        .and_then(Value::as_str)
        .is_some_and(|version| version == "2.0")
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

fn map_tool_call(name: &str, arguments: Value) -> Result<DaemonRequest, String> {
    match name {
        "rag_reload_config" => Ok(DaemonRequest::ReloadConfig),
        "status" | "rag_index_status" => Ok(DaemonRequest::Status {
            snapshot_id: value_string(&arguments, "snapshot_id"),
            worktree_root: value_string(&arguments, "worktree_root"),
        }),
        "index_workspace" | "rag_reindex" => Ok(DaemonRequest::IndexWorkspace {
            snapshot: rarag_core::snapshot::SnapshotKey::new(
                required_value(&arguments, "repo_root")?,
                required_value(&arguments, "worktree_root")?,
                required_value(&arguments, "git_sha")?,
                value_string(&arguments, "cargo_target")
                    .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string()),
                value_string(&arguments, "feature")
                    .unwrap_or_else(|| "default".to_string())
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>(),
                value_string(&arguments, "cfg_profile").unwrap_or_else(|| "dev".to_string()),
            ),
            workspace_root: required_value(&arguments, "workspace_root")?,
            max_body_bytes: value_string(&arguments, "max_body_bytes")
                .as_deref()
                .unwrap_or("80")
                .parse()
                .map_err(|err| format!("invalid max_body_bytes: {err}"))?,
        }),
        "query_context" | "find_examples" | "blast_radius" | "rag_query" | "rag_symbol_context"
        | "rag_examples" | "rag_blast_radius" => {
            let query_mode = match name {
                "find_examples" | "rag_examples" => QueryMode::FindExamples,
                "blast_radius" | "rag_blast_radius" => QueryMode::BlastRadius,
                "rag_symbol_context" => QueryMode::UnderstandSymbol,
                _ => parse_query_mode(&required_value(&arguments, "mode")?)?,
            };
            let payload = QueryPayload {
                snapshot_id: value_string(&arguments, "snapshot_id"),
                worktree_root: value_string(&arguments, "worktree_root"),
                query_mode,
                query_text: required_value(&arguments, "text")?,
                symbol_path: value_string(&arguments, "symbol_path"),
                limit: value_string(&arguments, "limit")
                    .map(|value| value.parse().map_err(|err| format!("invalid limit: {err}")))
                    .transpose()?,
                changed_paths: value_array_strings(&arguments, "changed_paths"),
            };
            if matches!(name, "blast_radius" | "rag_blast_radius") {
                Ok(DaemonRequest::BlastRadius(payload))
            } else {
                Ok(DaemonRequest::Query(payload))
            }
        }
        _ => Err(format!("unsupported tool {name}")),
    }
}

fn parse_query_mode(value: &str) -> Result<QueryMode, String> {
    match value {
        "understand-symbol" => Ok(QueryMode::UnderstandSymbol),
        "implement-adjacent" => Ok(QueryMode::ImplementAdjacent),
        "bounded-refactor" => Ok(QueryMode::BoundedRefactor),
        "find-examples" => Ok(QueryMode::FindExamples),
        "blast-radius" => Ok(QueryMode::BlastRadius),
        _ => Err(format!("unsupported mode {value}")),
    }
}

fn required_value(arguments: &Value, key: &str) -> Result<String, String> {
    value_string(arguments, key).ok_or_else(|| format!("missing required field {key}"))
}

fn value_string(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn value_array_strings(arguments: &Value, key: &str) -> Vec<String> {
    arguments
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn send_daemon_request(
    socket_path: &Path,
    request: &DaemonRequest,
) -> Result<rarag_core::daemon::DaemonResponse, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|err| err.to_string())?;
    let body = serde_json::to_vec(request).map_err(|err| err.to_string())?;
    write_framed_message(&mut stream, &body)?;
    let response = read_framed_message(&mut stream)?;
    serde_json::from_slice(&response).map_err(|err| err.to_string())
}

fn read_request_value(stream: &mut UnixStream) -> Result<Value, String> {
    read_request_value_with_limits(stream, LOCAL_IPC_MAX_MESSAGE_BYTES, LOCAL_IPC_READ_TIMEOUT)
}

fn read_request_value_with_limits(
    stream: &mut UnixStream,
    max_message_bytes: usize,
    read_timeout: std::time::Duration,
) -> Result<Value, String> {
    let deadline = Instant::now() + read_timeout;
    let mut body = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err("mcp request timed out".to_string());
        }
        stream
            .set_read_timeout(Some(deadline.saturating_duration_since(now)))
            .map_err(|err| err.to_string())?;
        match stream.read(&mut chunk) {
            Ok(0) => return serde_json::from_slice(&body).map_err(|err| err.to_string()),
            Ok(read) => {
                body.extend_from_slice(&chunk[..read]);
                if body.len() > max_message_bytes {
                    return Err(format!(
                        "mcp request too large: {} bytes exceeds limit {max_message_bytes}",
                        body.len(),
                    ));
                }
                match serde_json::from_slice(&body) {
                    Ok(value) => return Ok(value),
                    Err(err) if err.classify() == serde_json::error::Category::Eof => {}
                    Err(err) => return Err(err.to_string()),
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err("mcp request timed out".to_string());
            }
            Err(err) => return Err(err.to_string()),
        }
    }
}

fn write_response_value(stream: &mut UnixStream, response: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(response).map_err(|err| err.to_string())?;
    stream.write_all(&body).map_err(|err| err.to_string())
}

fn read_stdio_request_value<R: BufRead>(reader: &mut R) -> Result<Option<Value>, String> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|err| err.to_string())?;
        if read == 0 {
            if content_length.is_none() {
                return Ok(None);
            }
            return Err("unexpected EOF while reading MCP headers".to_string());
        }

        if line == "\n" || line == "\r\n" {
            break;
        }

        let header = line.trim_end_matches(['\r', '\n']);
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|err| format!("invalid Content-Length header: {err}"))?;
            content_length = Some(parsed);
        }
    }

    let content_length =
        content_length.ok_or_else(|| "missing Content-Length header in MCP stdio request".to_string())?;
    if content_length > LOCAL_IPC_MAX_MESSAGE_BYTES {
        return Err(format!(
            "mcp request too large: {content_length} bytes exceeds limit {LOCAL_IPC_MAX_MESSAGE_BYTES}"
        ));
    }

    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map(Some).map_err(|err| err.to_string())
}

fn write_stdio_response_value<W: Write>(writer: &mut W, response: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(response).map_err(|err| err.to_string())?;
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
        .map_err(|err| err.to_string())?;
    writer.write_all(&body).map_err(|err| err.to_string())?;
    writer.flush().map_err(|err| err.to_string())
}

pub fn daemon_socket_from_args(args: &[String], default_socket: &str) -> PathBuf {
    args.windows(2)
        .find(|window| window[0] == "--daemon-socket")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from(default_socket))
}

#[cfg(test)]
mod tests {
    use super::read_request_value_with_limits;
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    #[test]
    fn request_size_limit_rejects_oversized_incomplete_json() {
        let (mut reader, mut writer) = UnixStream::pair().expect("unix stream pair");
        writer
            .write_all(b"[                                ")
            .expect("write oversized incomplete payload");
        writer
            .shutdown(std::net::Shutdown::Write)
            .expect("shutdown writer");

        let err =
            read_request_value_with_limits(&mut reader, 16, Duration::from_secs(5)).unwrap_err();

        assert!(err.contains("too large"), "error was: {err}");
    }
}
