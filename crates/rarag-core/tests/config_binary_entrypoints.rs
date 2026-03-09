use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::{fs, os::unix::fs::PermissionsExt};

use rarag_core::config_loader::load_app_config;
use tempfile::tempdir;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn run_binary(binary: &str, args: &[&str], xdg_config_home: &std::path::Path) -> String {
    let output = Command::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg(binary)
        .arg("--")
        .args(args)
        .env("XDG_CONFIG_HOME", xdg_config_home)
        .env_remove("RARAG_CONFIG")
        .current_dir(workspace_root())
        .output()
        .expect("run binary");

    assert!(
        output.status.success(),
        "binary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn cli_uses_default_config_without_file() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");

    let stdout = run_binary("rarag", &["--print-config"], dir.path());

    assert!(stdout.contains("binary=rarag"), "stdout was: {stdout}");
    assert!(
        stdout.contains("default_json=false"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("embedding_model=text-embedding-3-small"),
        "stdout was: {stdout}"
    );
}

#[test]
fn daemon_accepts_explicit_config_path() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");
    let config_path = dir.path().join("rarag.toml");

    std::fs::write(
        &config_path,
        r#"
[daemon]
socket_path = "/tmp/rarag-daemon.sock"
"#,
    )
    .expect("write config");

    let stdout = run_binary(
        "raragd",
        &[
            "--config",
            config_path.to_str().expect("config path"),
            "--print-config",
        ],
        dir.path(),
    );

    assert!(stdout.contains("binary=raragd"), "stdout was: {stdout}");
    assert!(
        stdout.contains("socket_path=/tmp/rarag-daemon.sock"),
        "stdout was: {stdout}"
    );
}

#[test]
fn mcp_and_daemon_use_distinct_default_sockets() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");
    let config_path = dir.path().join("rarag.toml");

    std::fs::write(
        &config_path,
        r#"
[runtime]
socket_path = "/tmp/raragd.sock"
"#,
    )
    .expect("write config");

    let daemon_stdout = run_binary(
        "raragd",
        &[
            "--config",
            config_path.to_str().expect("config path"),
            "--print-config",
        ],
        dir.path(),
    );
    let mcp_stdout = run_binary(
        "rarag-mcp",
        &[
            "--config",
            config_path.to_str().expect("config path"),
            "--print-config",
        ],
        dir.path(),
    );

    assert!(
        daemon_stdout.contains("socket_path=/tmp/raragd.sock"),
        "stdout was: {daemon_stdout}"
    );
    assert!(
        mcp_stdout.contains("socket_path=/tmp/raragd-mcp.sock"),
        "stdout was: {mcp_stdout}"
    );
}

#[test]
fn example_toml_matches_resolved_shape() {
    let config = load_app_config(Some(&workspace_root().join("examples/rarag.example.toml")))
        .expect("load example config");

    assert_eq!(config.embeddings.base_url, "https://api.openai.com/v1");
    assert_eq!(config.embeddings.endpoint_path, "/embeddings");
    assert_eq!(config.embeddings.model, "text-embedding-3-small");
    assert_eq!(config.lancedb.table, "rarag_chunks");
}

#[test]
fn daemon_defaults_avoid_shared_tmp_runtime_socket() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");

    let output = Command::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("raragd")
        .arg("--")
        .arg("--print-config")
        .env_remove("XDG_RUNTIME_DIR")
        .env("HOME", dir.path())
        .env_remove("RARAG_CONFIG")
        .current_dir(workspace_root())
        .output()
        .expect("run binary");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        !stdout.contains("socket_path=/tmp/rarag/"),
        "stdout was: {stdout}"
    );
}

#[test]
fn daemon_and_mcp_default_to_private_home_runtime_root_without_xdg_runtime() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let dir = tempdir().expect("tempdir");

    let daemon_output = Command::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("raragd")
        .arg("--")
        .arg("--print-config")
        .env_remove("XDG_RUNTIME_DIR")
        .env("HOME", dir.path())
        .env_remove("RARAG_CONFIG")
        .current_dir(workspace_root())
        .output()
        .expect("run daemon binary");
    assert!(
        daemon_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&daemon_output.stderr)
    );
    let daemon_stdout = String::from_utf8(daemon_output.stdout).expect("utf8 stdout");
    let expected_runtime = dir
        .path()
        .join(".local/state/rarag/run/raragd.sock")
        .display()
        .to_string();
    assert!(
        daemon_stdout.contains(&format!("socket_path={expected_runtime}")),
        "stdout was: {daemon_stdout}"
    );
    let runtime_dir = dir.path().join(".local/state/rarag/run");
    let mode = fs::metadata(&runtime_dir)
        .expect("runtime dir metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700);

    let mcp_output = Command::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("rarag-mcp")
        .arg("--")
        .arg("--print-config")
        .env_remove("XDG_RUNTIME_DIR")
        .env("HOME", dir.path())
        .env_remove("RARAG_CONFIG")
        .current_dir(workspace_root())
        .output()
        .expect("run mcp binary");
    assert!(
        mcp_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&mcp_output.stderr)
    );
    let mcp_stdout = String::from_utf8(mcp_output.stdout).expect("utf8 stdout");
    let expected_mcp_runtime = dir
        .path()
        .join(".local/state/rarag/run/rarag-mcp.sock")
        .display()
        .to_string();
    assert!(
        mcp_stdout.contains(&format!("socket_path={expected_mcp_runtime}")),
        "stdout was: {mcp_stdout}"
    );
}
