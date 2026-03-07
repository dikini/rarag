use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

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
    assert_eq!(config.qdrant.collection, "rarag_chunks");
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
