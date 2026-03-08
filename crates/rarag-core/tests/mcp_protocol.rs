use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use rarag_core::daemon::DaemonRequest;
use rarag_core::ipc::{LOCAL_IPC_MAX_MESSAGE_BYTES, read_framed_message, write_framed_message};
use rarag_core::snapshot::SnapshotKey;
use serde_json::{Value, json};
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
    let mut built = built
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    workspace_root().join("target/debug").join(name)
}

#[allow(clippy::zombie_processes)]
fn spawn_server(binary: &str, args: &[&str], socket_path: &Path, probe: Value) -> Child {
    let runtime_root = socket_path.parent().expect("socket parent").to_path_buf();
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
        let ready = if binary == "raragd" {
            daemon_json_request(socket_path, &probe).is_ok()
        } else {
            json_request(socket_path, &probe).is_ok()
        };
        if socket_path.exists() && ready {
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

fn json_request(socket_path: &Path, body: &Value) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|err| err.to_string())?;
    let bytes = serde_json::to_vec(body).map_err(|err| err.to_string())?;
    stream.write_all(&bytes).map_err(|err| err.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|err| err.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&response).map_err(|err| err.to_string())
}

fn daemon_request(socket_path: &Path, body: &DaemonRequest) -> Result<Value, String> {
    let body = serde_json::to_value(body).map_err(|err| err.to_string())?;
    daemon_json_request(socket_path, &body)
}

fn daemon_json_request(socket_path: &Path, body: &Value) -> Result<Value, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|err| err.to_string())?;
    let bytes = serde_json::to_vec(body).map_err(|err| err.to_string())?;
    write_framed_message(&mut stream, &bytes)?;
    let response = read_framed_message(&mut stream)?;
    serde_json::from_slice(&response).map_err(|err| err.to_string())
}

fn raw_json_response(stream: &mut UnixStream) -> Result<Value, String> {
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&response).map_err(|err| err.to_string())
}

struct StdioMcpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioMcpClient {
    fn spawn(args: &[&str], runtime_root: &Path) -> Self {
        let mut child = Command::new(ensure_binary("rarag-mcp"))
            .args(args)
            .env("XDG_RUNTIME_DIR", runtime_root)
            .env("XDG_STATE_HOME", runtime_root)
            .env("XDG_CACHE_HOME", runtime_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn stdio mcp");
        let stdin = child.stdin.take().expect("mcp stdin");
        let stdout = child.stdout.take().expect("mcp stdout");
        Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    fn request(&mut self, value: &Value) -> Value {
        let body = serde_json::to_vec(value).expect("serialize request");
        self.stdin
            .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
            .expect("write headers");
        self.stdin.write_all(&body).expect("write body");
        self.stdin.flush().expect("flush stdin");
        read_stdio_frame(&mut self.stdout).expect("read stdio response")
    }
}

impl Drop for StdioMcpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_stdio_frame(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(|err| err.to_string())?;
        if read == 0 {
            return Err("unexpected EOF while reading stdio frame headers".to_string());
        }
        if line == "\n" || line == "\r\n" {
            break;
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("Content-Length")
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|err| format!("invalid content length: {err}"))?,
            );
        }
    }
    let content_length = content_length.ok_or_else(|| "missing content length".to_string())?;
    let mut body = vec![0_u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|err| err.to_string())?;
    serde_json::from_slice(&body).map_err(|err| err.to_string())
}

#[test]
fn standard_client_can_initialize_and_call_rag_tools() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let mcp_socket = dir.path().join("rarag-mcp.sock");
    let snapshot_worktree = "/repo/.worktrees/mcp-protocol";

    let initialize_probe = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "compat-test", "version": "0.1.0" }
        }
    });

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
            "--test-memory-vector-store",
        ],
        &daemon_socket,
        json!({
            "kind": "status",
            "snapshot_id": null,
            "worktree_root": "/tmp/probe-worktree"
        }),
    );

    let snapshot = SnapshotKey::new(
        "/repo",
        snapshot_worktree,
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["default"],
        "dev",
    );
    let index_response = daemon_request(
        &daemon_socket,
        &DaemonRequest::IndexWorkspace {
            snapshot,
            workspace_root: fixture_root().display().to_string(),
            max_body_bytes: 80,
        },
    )
    .expect("index workspace");
    assert_eq!(index_response["kind"], "indexed");

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
        initialize_probe.clone(),
    );

    let initialize = json_request(&mcp_socket, &initialize_probe).expect("initialize");
    assert_eq!(initialize["jsonrpc"], "2.0");
    assert_eq!(initialize["id"], 1);
    assert_eq!(initialize["result"]["protocolVersion"], "2025-03-26");

    let list_tools = json_request(
        &mcp_socket,
        &json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
    )
    .expect("list tools");
    let tool_names: Vec<_> = list_tools["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    assert!(tool_names.contains(&"rag_symbol_context"));

    let call_tool = json_request(
        &mcp_socket,
        &json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "rag_symbol_context",
                "arguments": {
                    "worktree_root": snapshot_worktree,
                    "text": "example_sum"
                }
            }
        }),
    )
    .expect("call tool");
    assert_eq!(call_tool["jsonrpc"], "2.0");
    assert_eq!(call_tool["id"], 3);
    assert!(
        call_tool["result"]["structuredContent"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = mcp.kill();
    let _ = mcp.wait();
}

#[test]
fn stdio_client_can_initialize_and_call_rag_tools() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let snapshot_worktree = "/repo/.worktrees/mcp-stdio-protocol";

    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": { "name": "stdio-test", "version": "0.1.0" }
        }
    });

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
            "--test-memory-vector-store",
        ],
        &daemon_socket,
        json!({
            "kind": "status",
            "snapshot_id": null,
            "worktree_root": "/tmp/probe-worktree"
        }),
    );

    let snapshot = SnapshotKey::new(
        "/repo",
        snapshot_worktree,
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["default"],
        "dev",
    );
    let index_response = daemon_request(
        &daemon_socket,
        &DaemonRequest::IndexWorkspace {
            snapshot,
            workspace_root: fixture_root().display().to_string(),
            max_body_bytes: 80,
        },
    )
    .expect("index workspace");
    assert_eq!(index_response["kind"], "indexed");

    let mut mcp = StdioMcpClient::spawn(
        &[
            "serve-stdio",
            "--daemon-socket",
            daemon_socket.to_str().expect("daemon socket"),
        ],
        dir.path(),
    );

    let init_response = mcp.request(&initialize);
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);

    let tools_response = mcp.request(&json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }));
    let tool_names: Vec<_> = tools_response["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    assert!(tool_names.contains(&"rag_symbol_context"));

    let call_response = mcp.request(&json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "rag_symbol_context",
            "arguments": {
                "worktree_root": snapshot_worktree,
                "text": "example_sum"
            }
        }
    }));
    assert!(
        call_response["result"]["structuredContent"]["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );

    let _ = daemon.kill();
    let _ = daemon.wait();
}

#[test]
fn rejects_oversized_socket_request() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let mcp_socket = dir.path().join("rarag-mcp.sock");

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
            "--test-memory-vector-store",
        ],
        &daemon_socket,
        json!({
            "kind": "status",
            "snapshot_id": null,
            "worktree_root": "/tmp/probe-worktree"
        }),
    );
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
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "compat-test", "version": "0.1.0" }
            }
        }),
    );

    let mut stream = UnixStream::connect(&mcp_socket).expect("connect mcp socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .expect("set read timeout");
    let mut oversized = vec![b'['];
    oversized.extend(std::iter::repeat_n(
        b' ',
        LOCAL_IPC_MAX_MESSAGE_BYTES + 1 - oversized.len(),
    ));
    let mut writer = stream.try_clone().expect("clone stream");
    let writer_handle = thread::spawn(move || {
        for chunk in oversized.chunks(4096) {
            match writer.write_all(chunk) {
                Ok(()) => {}
                Err(err)
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                    ) =>
                {
                    break;
                }
                Err(err) => panic!("write oversized body: {err}"),
            }
        }
        let _ = writer.shutdown(std::net::Shutdown::Write);
    });
    let response = raw_json_response(&mut stream);
    writer_handle.join().expect("join oversized writer");
    match response {
        Ok(payload) => {
            let error = payload["error"].as_str().unwrap_or_default();
            assert!(
                error.contains("too large") || error.contains("timed out"),
                "response was: {payload}"
            );
        }
        Err(err) => {
            assert!(
                err.contains("Connection reset by peer"),
                "unexpected oversized request error: {err}"
            );
        }
    }

    let _ = mcp.kill();
    let _ = daemon.kill();
}

#[test]
fn times_out_incomplete_socket_request() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let mcp_socket = dir.path().join("rarag-mcp.sock");

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
            "--test-memory-vector-store",
        ],
        &daemon_socket,
        json!({
            "kind": "status",
            "snapshot_id": null,
            "worktree_root": "/tmp/probe-worktree"
        }),
    );
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
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "compat-test", "version": "0.1.0" }
            }
        }),
    );

    let mut stream = UnixStream::connect(&mcp_socket).expect("connect mcp socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set read timeout");
    stream
        .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"initialize""#)
        .expect("write partial json");
    let response = raw_json_response(&mut stream).expect("read timeout response");
    assert!(
        response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("timed out"),
        "response was: {response}"
    );

    let _ = mcp.kill();
    let _ = daemon.kill();
}

#[test]
fn times_out_slow_drip_socket_request() {
    let dir = tempdir().expect("tempdir");
    let daemon_socket = dir.path().join("raragd.sock");
    let mcp_socket = dir.path().join("rarag-mcp.sock");

    let mut daemon = spawn_server(
        "raragd",
        &[
            "serve",
            "--socket",
            daemon_socket.to_str().expect("daemon socket"),
            "--test-deterministic-embeddings",
            "--test-memory-vector-store",
        ],
        &daemon_socket,
        json!({
            "kind": "status",
            "snapshot_id": null,
            "worktree_root": "/tmp/probe-worktree"
        }),
    );
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
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "compat-test", "version": "0.1.0" }
            }
        }),
    );

    let mut stream = UnixStream::connect(&mcp_socket).expect("connect mcp socket");
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .expect("set read timeout");
    let mut writer = stream.try_clone().expect("clone stream");
    let slow_bytes = br#"{"jsonrpc":"2.0","id":1,"method":"initialize""#;
    let started = Instant::now();
    let writer_handle = thread::spawn(move || {
        for byte in slow_bytes.iter().copied() {
            if writer.write_all(&[byte]).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(250));
        }
    });

    let response = raw_json_response(&mut stream).expect("read slow-drip timeout response");
    writer_handle.join().expect("join slow writer");

    assert!(
        response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("timed out"),
        "response was: {response}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "slow-drip timeout took too long: {:?}",
        started.elapsed()
    );

    let _ = mcp.kill();
    let _ = daemon.kill();
}
