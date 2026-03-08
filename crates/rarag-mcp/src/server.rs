use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use rarag_core::daemon::{DaemonRequest, QueryPayload};
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
    stream.write_all(&body).map_err(|err| err.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|err| err.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&response).map_err(|err| err.to_string())
}

fn read_request_value(stream: &mut UnixStream) -> Result<Value, String> {
    let mut body = Vec::new();
    stream
        .read_to_end(&mut body)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map_err(|err| err.to_string())
}

fn write_response_value(stream: &mut UnixStream, response: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(response).map_err(|err| err.to_string())?;
    stream.write_all(&body).map_err(|err| err.to_string())
}

pub fn daemon_socket_from_args(args: &[String], default_socket: &str) -> PathBuf {
    args.windows(2)
        .find(|window| window[0] == "--daemon-socket")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from(default_socket))
}
