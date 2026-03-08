use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use rarag_core::daemon::{DaemonRequest, DaemonResponse, QueryPayload};
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
    stream.write_all(&body).expect("write request");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown write");

    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    serde_json::from_slice(&response).expect("deserialize response")
}

fn send_request_if_ready(
    socket_path: &Path,
    request: &DaemonRequest,
) -> Result<DaemonResponse, std::io::Error> {
    let mut stream = UnixStream::connect(socket_path)?;
    let body = serde_json::to_vec(request).expect("serialize request");
    stream.write_all(&body).expect("write request");
    stream
        .shutdown(std::net::Shutdown::Write)
        .expect("shutdown write");

    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    Ok(serde_json::from_slice(&response).expect("deserialize response"))
}

#[allow(clippy::zombie_processes)]
fn spawn_daemon(socket_path: &Path) -> Child {
    let daemon_bin = workspace_root().join("target/debug/raragd");
    let runtime_root = socket_path.parent().expect("socket parent");
    let mut child = Command::new(&daemon_bin)
        .arg("serve")
        .arg("--socket")
        .arg(socket_path)
        .arg("--test-deterministic-embeddings")
        .arg("--test-memory-vector-store")
        .env("XDG_RUNTIME_DIR", runtime_root)
        .env("XDG_STATE_HOME", runtime_root)
        .env("XDG_CACHE_HOME", runtime_root)
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
    let body = serde_json::to_string(&DaemonRequest::Status {
        snapshot_id: Some("snapshot-1".to_string()),
        worktree_root: None,
    })
    .expect("serialize request");

    assert!(body.contains("\"kind\":\"status\""));
    assert!(body.contains("\"snapshot_id\":\"snapshot-1\""));
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
fn daemon_uses_configured_qdrant_endpoint() {
    let dir = tempdir().expect("tempdir");
    let socket_path = dir.path().join("raragd.sock");
    let config_path = dir.path().join("rarag.toml");
    std::fs::write(
        &config_path,
        r#"
[qdrant]
endpoint = "http://127.0.0.1:1"
collection = "rarag_chunks"
"#,
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
                "/repo/.worktrees/qdrant-config",
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
        DaemonResponse::Error(error) => {
            assert!(
                error.message.contains("transport")
                    || error.message.contains("connection")
                    || error.message.contains("tcp"),
                "message was: {:?}",
                error
            );
        }
        other => panic!("expected qdrant connection error, got {other:?}"),
    }

    let shutdown = send_request(&socket_path, &DaemonRequest::Shutdown);
    assert!(matches!(shutdown, DaemonResponse::Ack));
    let status = child.wait().expect("wait for daemon");
    assert!(status.success(), "daemon exited with {status}");
}
