use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use rarag_core::daemon::{DaemonRequest, QueryPayload};
use rarag_core::retrieval::{QueryMode, WorkflowPhase};
use rarag_core::unix_socket::prepare_socket_path;
use serde_json::Value;

use crate::tools::{McpRequest, McpResponse, tool_definitions};

pub fn serve(socket_path: &Path, daemon_socket: &Path) -> Result<(), String> {
    prepare_socket_path(socket_path)?;
    let listener = UnixListener::bind(socket_path).map_err(|err| err.to_string())?;

    for stream in listener.incoming() {
        let mut stream = stream.map_err(|err| err.to_string())?;
        let response = match read_request(&mut stream) {
            Ok(request) => handle_request(request, daemon_socket),
            Err(err) => McpResponse::Error { message: err },
        };
        write_response(&mut stream, &response)?;
    }

    Ok(())
}

fn handle_request(request: McpRequest, daemon_socket: &Path) -> McpResponse {
    match request {
        McpRequest::ListTools => McpResponse::Tools {
            tools: tool_definitions(),
        },
        McpRequest::CallTool { name, arguments } => match map_tool_call(&name, arguments) {
            Ok(request) => match send_daemon_request(daemon_socket, &request) {
                Ok(response) => McpResponse::CallResult {
                    result: serde_json::to_value(response).expect("serialize daemon response"),
                },
                Err(err) => McpResponse::Error { message: err },
            },
            Err(err) => McpResponse::Error { message: err },
        },
    }
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
                workflow_phase: parse_workflow_phase(&required_value(&arguments, "phase")?)?,
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

fn parse_workflow_phase(value: &str) -> Result<WorkflowPhase, String> {
    match value {
        "spec" => Ok(WorkflowPhase::Spec),
        "plan" => Ok(WorkflowPhase::Plan),
        "write-tests" | "tests" => Ok(WorkflowPhase::WriteTests),
        "write-code" | "code" => Ok(WorkflowPhase::WriteCode),
        "verify" => Ok(WorkflowPhase::Verify),
        "review" => Ok(WorkflowPhase::Review),
        "fix" => Ok(WorkflowPhase::Fix),
        _ => Err(format!("unsupported phase {value}")),
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

fn read_request(stream: &mut UnixStream) -> Result<McpRequest, String> {
    let mut body = Vec::new();
    stream
        .read_to_end(&mut body)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map_err(|err| err.to_string())
}

fn write_response(stream: &mut UnixStream, response: &McpResponse) -> Result<(), String> {
    let body = serde_json::to_vec(response).map_err(|err| err.to_string())?;
    stream.write_all(&body).map_err(|err| err.to_string())
}

pub fn daemon_socket_from_args(args: &[String], default_socket: &str) -> PathBuf {
    args.windows(2)
        .find(|window| window[0] == "--daemon-socket")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from(default_socket))
}
