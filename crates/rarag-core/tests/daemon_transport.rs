use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::{io::Cursor, iter};

use rarag_core::daemon::{DaemonRequest, DaemonResponse, QueryPayload};
use rarag_core::ipc::{DAEMON_MAX_MESSAGE_BYTES, read_framed_message, write_framed_message};
use rarag_core::retrieval::QueryMode;
use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_root() -> PathBuf {
    workspace_root().join("tests/fixtures/mini_repo")
}

fn send_request(socket_path: &Path, request: &DaemonRequest) -> DaemonResponse {
    let mut stream = UnixStream::connect(socket_path).expect("connect unix socket");
    let body = serde_json::to_vec(request).expect("serialize request");
    write_framed_message(&mut stream, &body).expect("write framed request");
    let response = read_framed_message(&mut stream).expect("read framed response");
    serde_json::from_slice(&response).expect("deserialize response")
}

fn send_request_if_ready(
    socket_path: &Path,
    request: &DaemonRequest,
) -> Result<DaemonResponse, std::io::Error> {
    let mut stream = UnixStream::connect(socket_path)?;
    let body = serde_json::to_vec(request).expect("serialize request");
    write_framed_message(&mut stream, &body).expect("write framed request");
    let response = read_framed_message(&mut stream).expect("read framed response");
    Ok(serde_json::from_slice(&response).expect("deserialize response"))
}

#[allow(clippy::zombie_processes)]
fn spawn_daemon(socket_path: &Path) -> Child {
    spawn_daemon_with_args(socket_path, &[])
}

#[allow(clippy::zombie_processes)]
fn spawn_daemon_with_args(socket_path: &Path, extra_args: &[&str]) -> Child {
    let daemon_bin = workspace_root().join("target/debug/raragd");
    let runtime_root = socket_path.parent().expect("socket parent");
    let mut command = Command::new(&daemon_bin);
    command
        .arg("serve")
        .arg("--socket")
        .arg(socket_path)
        .arg("--test-deterministic-embeddings")
        .arg("--test-memory-vector-store");
    command.args(extra_args);
    let mut child = command
        .env("XDG_RUNTIME_DIR", runtime_root)
        .env("XDG_STATE_HOME", runtime_root)
        .env("XDG_CACHE_HOME", runtime_root)
        .current_dir(workspace_root())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn daemon");

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if socket_path.exists() {
            return child;
        }
        if let Some(status) = child.try_wait().expect("check daemon status") {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            panic!("daemon exited early with {status}: {stderr}");
        }
        thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    panic!("daemon socket was not created");
}

#[test]
fn serializes_unix_socket_requests() {
    let payload = serde_json::to_vec(&DaemonRequest::Status {
        snapshot_id: Some("snapshot-1".to_string()),
        worktree_root: None,
    })
    .expect("serialize request");
    let mut framed = Vec::new();
    framed.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    framed.extend_from_slice(&payload);

    assert_eq!(&framed[..4], &(payload.len() as u32).to_be_bytes());
    assert_eq!(&framed[4..], payload.as_slice());
}

#[test]
fn serializes_reload_request() {
    let body = serde_json::to_string(&DaemonRequest::ReloadConfig).expect("serialize request");

    assert!(body.contains("\"kind\":\"reload-config\""));
}

#[test]
fn read_framed_response_accepts_large_payload() {
    let payload: Vec<u8> = iter::repeat_n(b'x', DAEMON_MAX_MESSAGE_BYTES + 1024).collect();
    let framed = rarag_core::ipc::encode_framed_message(&payload).expect("encode large payload");
    let mut reader = Cursor::new(framed);

    let decoded = read_framed_message(&mut reader).expect("large framed response should decode");

    assert_eq!(decoded.len(), payload.len());
    assert_eq!(decoded, payload);
}

#[test]
fn requests_require_snapshot_or_unambiguous_worktree() {
    let err = QueryPayload {
        snapshot_id: None,
        worktree_root: None,
        query_mode: QueryMode::UnderstandSymbol,
        query_text: "example_sum".to_string(),
        symbol_path: None,
        limit: None,
        changed_paths: Vec::new(),
        include_history: false,
        history_max_nodes: None,
        eval_task_id: None,
    }
    .validate_locator()
    .expect_err("missing locator should fail");

    assert!(err.contains("snapshot_id or worktree_root"));
}

#[test]
fn daemon_roundtrip_serves_query_payload() {
    let dir = tempdir().expect("tempdir");
    let socket_path = dir.path().join("raragd.sock");
    let snapshot = SnapshotKey::new(
        "/repo",
        "/repo/.worktrees/daemon-transport",
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["default"],
        "dev",
    );
    let mut daemon = spawn_daemon(&socket_path);

    let index_response = send_request(
        &socket_path,
        &DaemonRequest::IndexWorkspace {
            snapshot: snapshot.clone(),
            workspace_root: fixture_root().display().to_string(),
            max_body_bytes: 80,
        },
    );
    match index_response {
        DaemonResponse::Indexed(payload) => {
            assert_eq!(payload.snapshot_id, snapshot.id());
            assert!(payload.chunk_count > 0);
        }
        other => panic!("unexpected index response: {other:?}"),
    }

    let query_response = send_request(
        &socket_path,
        &DaemonRequest::Query(QueryPayload {
            snapshot_id: None,
            worktree_root: Some(snapshot.worktree_root.clone()),
            query_mode: QueryMode::UnderstandSymbol,
            query_text: "example_sum".to_string(),
            symbol_path: Some("mini_repo::example_sum".to_string()),
            limit: Some(4),
            changed_paths: Vec::new(),
            include_history: false,
            history_max_nodes: None,
            eval_task_id: None,
        }),
    );
    match query_response {
        DaemonResponse::Query(payload) => {
            assert!(!payload.items.is_empty());
            assert_eq!(
                payload.items[0].chunk.symbol_path.as_deref(),
                Some("mini_repo::example_sum")
            );
        }
        other => panic!("unexpected query response: {other:?}"),
    }

    let shutdown = send_request(&socket_path, &DaemonRequest::Shutdown);
    assert!(matches!(shutdown, DaemonResponse::Ack));

    let status = daemon.wait().expect("wait for daemon");
    assert!(status.success(), "daemon exited with {status}");
}

#[test]
fn rejects_oversized_requests() {
    let dir = tempdir().expect("tempdir");
    let socket_path = dir.path().join("raragd.sock");
    let mut daemon = spawn_daemon(&socket_path);

    let mut stream = UnixStream::connect(&socket_path).expect("connect unix socket");
    let oversize = (DAEMON_MAX_MESSAGE_BYTES as u32) + 1;
    stream
        .write_all(&oversize.to_be_bytes())
        .expect("write oversized frame header");
    let response = read_framed_message(&mut stream).expect("read daemon error response");
    let response: DaemonResponse = serde_json::from_slice(&response).expect("deserialize response");
    match response {
        DaemonResponse::Error(error) => assert!(error.message.contains("too large")),
        other => panic!("unexpected response: {other:?}"),
    }

    let shutdown = send_request(&socket_path, &DaemonRequest::Shutdown);
    assert!(matches!(shutdown, DaemonResponse::Ack));
    let status = daemon.wait().expect("wait for daemon");
    assert!(status.success(), "daemon exited with {status}");
}

#[test]
fn times_out_incomplete_requests() {
    let dir = tempdir().expect("tempdir");
    let socket_path = dir.path().join("raragd.sock");
    let mut daemon = spawn_daemon(&socket_path);

    let mut stream = UnixStream::connect(&socket_path).expect("connect unix socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set read timeout");
    stream
        .write_all(&16_u32.to_be_bytes())
        .expect("write frame header");
    stream.write_all(b"{").expect("write partial body");

    let response = read_framed_message(&mut stream).expect("read daemon timeout response");
    let response: DaemonResponse = serde_json::from_slice(&response).expect("deserialize response");
    match response {
        DaemonResponse::Error(error) => assert!(error.message.contains("timed out")),
        other => panic!("unexpected response: {other:?}"),
    }

    let shutdown = send_request(&socket_path, &DaemonRequest::Shutdown);
    assert!(matches!(shutdown, DaemonResponse::Ack));
    let status = daemon.wait().expect("wait for daemon");
    assert!(status.success(), "daemon exited with {status}");
}

#[test]
fn daemon_uses_configured_lancedb_db_root() {
    let dir = tempdir().expect("tempdir");
    let socket_path = dir.path().join("raragd.sock");
    let config_path = dir.path().join("rarag.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
[lancedb]
db_root = "{}"
table = "rarag_chunks_custom"
"#,
            dir.path().join("lancedb-store").display()
        ),
    )
    .expect("write config");

    let daemon_bin = workspace_root().join("target/debug/raragd");
    let runtime_root = socket_path.parent().expect("socket parent");
    let mut child = Command::new(&daemon_bin)
        .arg("serve")
        .arg("--config")
        .arg(&config_path)
        .arg("--socket")
        .arg(&socket_path)
        .arg("--test-deterministic-embeddings")
        .env("XDG_RUNTIME_DIR", runtime_root)
        .env("XDG_STATE_HOME", runtime_root)
        .env("XDG_CACHE_HOME", runtime_root)
        .current_dir(workspace_root())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn daemon");

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if socket_path.exists()
            && send_request_if_ready(
                &socket_path,
                &DaemonRequest::Status {
                    snapshot_id: None,
                    worktree_root: Some("/tmp/probe-worktree".to_string()),
                },
            )
            .is_ok()
        {
            break;
        }
        if let Some(status) = child.try_wait().expect("check daemon status") {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            panic!("daemon exited early with {status}: {stderr}");
        }
        thread::sleep(Duration::from_millis(25));
    }
    assert!(socket_path.exists(), "daemon socket was not created");

    let response = send_request(
        &socket_path,
        &DaemonRequest::IndexWorkspace {
            snapshot: SnapshotKey::new(
                "/repo",
                "/repo/.worktrees/lancedb-config",
                "abc123",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ),
            workspace_root: fixture_root().display().to_string(),
            max_body_bytes: 80,
        },
    );

    match response {
        DaemonResponse::Indexed(payload) => {
            assert_eq!(
                payload.snapshot_id,
                "/repo|/repo/.worktrees/lancedb-config|abc123|x86_64-unknown-linux-gnu|default|dev"
            );
            assert!(payload.chunk_count > 0);
        }
        other => panic!("expected successful index response, got {other:?}"),
    }

    let shutdown = send_request(&socket_path, &DaemonRequest::Shutdown);
    assert!(matches!(shutdown, DaemonResponse::Ack));
    let status = child.wait().expect("wait for daemon");
    assert!(status.success(), "daemon exited with {status}");
}

#[test]
fn reload_failure_keeps_old_config() {
    let dir = tempdir().expect("tempdir");
    let socket_path = dir.path().join("raragd.sock");
    let config_path = dir.path().join("rarag.toml");
    std::fs::write(
        &config_path,
        r#"
[observability]
enabled = false
verbosity = "off"
"#,
    )
    .expect("write config");
    let mut daemon = spawn_daemon_with_args(
        &socket_path,
        &["--config", config_path.to_str().expect("config path")],
    );

    std::fs::write(
        &config_path,
        r#"
[observability
enabled = true
"#,
    )
    .expect("write invalid config");

    let response = send_request(&socket_path, &DaemonRequest::ReloadConfig);
    match response {
        DaemonResponse::Error(error) => {
            assert!(
                error.message.contains("failed to parse config"),
                "unexpected error: {}",
                error.message
            );
        }
        other => panic!("unexpected reload response: {other:?}"),
    }

    let status_response = send_request(
        &socket_path,
        &DaemonRequest::Status {
            snapshot_id: None,
            worktree_root: Some("/repo/.worktrees/reload-still-alive".to_string()),
        },
    );
    assert!(
        matches!(status_response, DaemonResponse::Status(_)),
        "unexpected status response after failed reload: {status_response:?}"
    );

    let shutdown = send_request(&socket_path, &DaemonRequest::Shutdown);
    assert!(matches!(shutdown, DaemonResponse::Ack));
    let status = daemon.wait().expect("wait for daemon");
    assert!(status.success(), "daemon exited with {status}");
}
