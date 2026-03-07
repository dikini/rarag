use std::path::Path;
use std::sync::{Mutex, OnceLock};

use rarag_core::config_loader::load_app_config;
use tempfile::tempdir;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn write_config(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().expect("config parent")).expect("create config dir");
    std::fs::write(path, body).expect("write config");
}

#[test]
fn prefers_explicit_config_path() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = tempdir().expect("tempdir");
    let explicit = dir.path().join("explicit.toml");
    let xdg_root = dir.path().join("xdg");

    write_config(
        &explicit,
        r#"
[embeddings]
base_url = "https://explicit.example.invalid/v1"
endpoint_path = "/embeddings"
model = "text-embedding-3-small"
api_key_env = "OPENAI_API_KEY"
dimensions = 1536
"#,
    );
    write_config(
        &xdg_root.join("rarag/rarag.toml"),
        r#"
[embeddings]
base_url = "https://xdg.example.invalid/v1"
endpoint_path = "/embeddings"
model = "text-embedding-3-small"
api_key_env = "OPENAI_API_KEY"
dimensions = 1536
"#,
    );

    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &xdg_root);
        std::env::remove_var("RARAG_CONFIG");
    }

    let config = load_app_config(Some(&explicit)).expect("load explicit config");

    assert_eq!(
        config.embeddings.base_url,
        "https://explicit.example.invalid/v1"
    );
}

#[test]
fn falls_back_to_xdg_config_path() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = tempdir().expect("tempdir");
    let xdg_root = dir.path().join("xdg");

    write_config(
        &xdg_root.join("rarag/rarag.toml"),
        r#"
[daemon]
socket_path = "/tmp/rarag-custom.sock"
"#,
    );

    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &xdg_root);
        std::env::remove_var("RARAG_CONFIG");
    }

    let config = load_app_config(None).expect("load xdg config");

    assert_eq!(
        config.daemon.as_ref().expect("daemon config").socket_path,
        "/tmp/rarag-custom.sock"
    );
}

#[test]
fn missing_config_uses_code_defaults() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = tempdir().expect("tempdir");

    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", dir.path().join("missing"));
        std::env::remove_var("RARAG_CONFIG");
    }

    let config = load_app_config(None).expect("load defaults without config");

    assert_eq!(config.embeddings.base_url, "https://api.openai.com/v1");
    assert_eq!(config.embeddings.endpoint_path, "/embeddings");
}

#[test]
fn toml_overrides_default_embedding_endpoint() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = tempdir().expect("tempdir");
    let explicit = dir.path().join("rarag.toml");

    write_config(
        &explicit,
        r#"
[embeddings]
base_url = "https://proxy.example.invalid/openai"
endpoint_path = "v1/embeddings"
model = "text-embedding-3-small"
api_key_env = "OPENAI_API_KEY"
dimensions = 1536
"#,
    );

    unsafe {
        std::env::remove_var("RARAG_CONFIG");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    let config = load_app_config(Some(&explicit)).expect("load explicit config");

    assert_eq!(
        config.embeddings.base_url,
        "https://proxy.example.invalid/openai"
    );
    assert_eq!(config.embeddings.endpoint_path, "v1/embeddings");
}
