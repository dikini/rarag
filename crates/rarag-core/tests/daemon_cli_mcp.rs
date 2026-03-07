use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
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

fn ensure_binary(name: &str) -> PathBuf {
    static BUILT: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    let built = BUILT.get_or_init(|| Mutex::new(Vec::new()));
    let mut built = built.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if !built.iter().any(|entry| entry == name) {
        let status = Command::new("cargo")
            .arg("build")
            .arg("-q")
            .arg("-p")
            .arg(name)
            .current_dir(workspace_root())
            .status()
            .expect("build binary");
        assert!(status.success(), "failed to build {name}");
        built.push(name.to_string());
    }
    let path = workspace_root().join("target/debug").join(name);
    assert!(path.exists(), "missing binary {}", path.display());
    path
}

#[allow(clippy::zombie_processes)]
fn spawn_server(binary: &str, args: &[&str], socket_path: &Path) -> Child {
    let runtime_root = socket_path
        .parent()
        .expect("socket parent directory")
        .to_path_buf();
    let mut child = Command::new(ensure_binary(binary))
        .args(args)
        .env("XDG_RUNTIME_DIR", &runtime_root)
        .env("XDG_STATE_HOME", &runtime_root)
        .env("XDG_CACHE_HOME", &runtime_root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn server");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if socket_path.exists() && probe_server(binary, socket_path).is_ok() {
            return child;
        }
        if let Some(status) = child.try_wait().expect("server status") {
            let mut stderr = String::new();
            if let Some(mut pipe) = child.stderr.take() {
                let _ = pipe.read_to_string(&mut stderr);
            }
            panic!("server exited early with {status}: {stderr}");
        }
        thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    panic!("server socket was not created");
}

fn probe_server(binary: &str, socket_path: &Path) -> Result<(), String> {
    let body = match binary {
        "raragd" => serde_json::json!({
            "kind": "status",
            "snapshot_id": null,
            "worktree_root": "/tmp/probe-worktree"
        }),
        "rarag-mcp" => serde_json::json!({
            "kind": "list_tools"
        }),
        _ => return Err(format!("unsupported probe binary {binary}")),
    };

    let mut stream = UnixStream::connect(socket_path).map_err(|err| err.to_string())?;
    stream
        .write_all(serde_json::to_vec(&body).map_err(|err| err.to_string())?.as_slice())
        .map_err(|err| err.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|err| err.to_string())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|err| err.to_string())?;
    if response.is_empty() {
        Err("empty probe response".to_string())
    } else {
        Ok(())
    }
}

fn run_cli(args: &[&str]) -> String {
    let output = Command::new(ensure_binary("rarag"))
        .args(args)
        .output()
        .expect("run cli");
    assert!(
        output.status.success(),
        "cli failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn mcp_request(socket_path: &Path, body: Value) -> Value {
    let mut stream = UnixStream::connect(socket_path).expect("connect mcp socket");
    stream
        .write_all(serde_json::to_vec(&body).expect("serialize mcp request").as_slice())
        .expect("write request");
    stream.shutdown(std::net::Shutdown::Write).expect("shutdown write");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).expect("read response");
    serde_json::from_slice(&response).expect("deserialize response")
}

#[test]
fn cli_parses_phase_and_mode_flags() {
    let stdout = run_cli(&[
        "query",
        "--socket",
        "/tmp/rarag-test.sock",
        "--worktree-root",
        "/tmp/worktree",
        "--mode",
        "bounded-refactor",
        "--phase",
        "review",
        "--text",
        "rename example_sum",
        "--dry-run-request",
    ]);

    assert!(stdout.contains("\"kind\": \"query\""));
    assert!(stdout.contains("\"query_mode\": \"BoundedRefactor\""));
    assert!(stdout.contains("\"workflow_phase\": \"Review\""));
}

#[test]
fn mcp_tool_names_match_contract() {
    let output = Command::new(ensure_binary("rarag-mcp"))
        .arg("--list-tools")
        .output()
        .expect("run mcp --list-tools");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");

    assert!(stdout.contains("index_workspace"));
    assert!(stdout.contains("query_context"));
    assert!(stdout.contains("find_examples"));
    assert!(stdout.contains("blast_radius"));
    assert!(stdout.contains("status"));
}

#[test]
fn cli_and_mcp_observe_same_snapshot_result() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let mcp_socket = dir.path().join("rarag-mcp.sock");
    let snapshot_worktree = "/repo/.worktrees/cli-mcp";

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
        ],
        &daemon_socket,
    );
    let index_stdout = run_cli(&[
        "index",
        "--socket",
        daemon_socket.to_str().expect("daemon socket"),
        "--workspace-root",
        fixture_root().to_str().expect("fixture root"),
        "--repo-root",
        "/repo",
        "--worktree-root",
        snapshot_worktree,
        "--git-sha",
        "abc123",
        "--json",
    ]);
    assert!(index_stdout.contains("indexed"));

    let cli_stdout = run_cli(&[
        "query",
        "--socket",
        daemon_socket.to_str().expect("daemon socket"),
        "--worktree-root",
        snapshot_worktree,
        "--mode",
        "understand-symbol",
        "--phase",
        "plan",
        "--text",
        "example_sum",
        "--symbol-path",
        "mini_repo::example_sum",
        "--json",
    ]);
    let cli_json: Value = serde_json::from_str(&cli_stdout).expect("cli json");
    let cli_top = cli_json["items"][0]["chunk"]["symbol_path"]
        .as_str()
        .expect("cli top symbol")
        .to_string();

    let mut mcp = spawn_server(
        "rarag-mcp",
        &[
            "serve",
            "--socket",
            mcp_socket.to_str().expect("mcp socket"),
            "--daemon-socket",
            daemon_socket.to_str().expect("daemon socket"),
        ],
        &mcp_socket,
    );
    let mcp_response = mcp_request(
        &mcp_socket,
        serde_json::json!({
            "kind": "call_tool",
            "name": "query_context",
            "arguments": {
                "worktree_root": snapshot_worktree,
                "mode": "understand-symbol",
                "phase": "plan",
                "text": "example_sum",
                "symbol_path": "mini_repo::example_sum"
            }
        }),
    );
    let mcp_top = mcp_response["result"]["items"][0]["chunk"]["symbol_path"]
        .as_str()
        .expect("mcp top symbol")
        .to_string();

    assert_eq!(cli_top, mcp_top);

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = mcp.kill();
    let _ = mcp.wait();
}

#[test]
fn cli_and_mcp_roundtrip_against_local_daemon() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let mcp_socket = dir.path().join("rarag-mcp.sock");
    let snapshot_worktree = "/repo/.worktrees/cli-mcp-roundtrip";

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
        ],
        &daemon_socket,
    );
    let _ = run_cli(&[
        "index",
        "--socket",
        daemon_socket.to_str().expect("daemon socket"),
        "--workspace-root",
        fixture_root().to_str().expect("fixture root"),
        "--repo-root",
        "/repo",
        "--worktree-root",
        snapshot_worktree,
        "--git-sha",
        "abc123",
        "--json",
    ]);

    let cli_stdout = run_cli(&[
        "blast-radius",
        "--socket",
        daemon_socket.to_str().expect("daemon socket"),
        "--worktree-root",
        snapshot_worktree,
        "--phase",
        "review",
        "--text",
        "example_sum",
        "--symbol-path",
        "mini_repo::example_sum",
        "--json",
    ]);
    let cli_json: Value = serde_json::from_str(&cli_stdout).expect("cli json");
    assert!(cli_json["items"].as_array().is_some_and(|items| !items.is_empty()));

    let mut mcp = spawn_server(
        "rarag-mcp",
        &[
            "serve",
            "--socket",
            mcp_socket.to_str().expect("mcp socket"),
            "--daemon-socket",
            daemon_socket.to_str().expect("daemon socket"),
        ],
        &mcp_socket,
    );
    let mcp_response = mcp_request(
        &mcp_socket,
        serde_json::json!({
            "kind": "call_tool",
            "name": "blast_radius",
            "arguments": {
                "worktree_root": snapshot_worktree,
                "phase": "review",
                "text": "example_sum",
                "symbol_path": "mini_repo::example_sum"
            }
        }),
    );
    assert!(mcp_response["result"]["items"]
        .as_array()
        .is_some_and(|items| !items.is_empty()));

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = mcp.kill();
    let _ = mcp.wait();
}
