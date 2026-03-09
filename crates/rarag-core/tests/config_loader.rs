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

#[test]
fn toml_overrides_rerank_and_observability() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = tempdir().expect("tempdir");
    let explicit = dir.path().join("rarag.toml");

    write_config(
        &explicit,
        r#"
[retrieval.rerank]
find_examples_example_like = 1.3
worktree_diff_blast_radius = 2.0

[retrieval.neighborhood]
same_file = 4.4
semantic_test_find_examples = 8.8

[observability]
enabled = true
verbosity = "summary"
"#,
    );

    unsafe {
        std::env::remove_var("RARAG_CONFIG");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    let config = load_app_config(Some(&explicit)).expect("load explicit config");

    assert_eq!(config.retrieval.rerank.find_examples_example_like, 1.3);
    assert_eq!(config.retrieval.rerank.worktree_diff_blast_radius, 2.0);
    assert_eq!(config.retrieval.neighborhood.same_file, 4.4);
    assert_eq!(
        config.retrieval.neighborhood.semantic_test_find_examples,
        8.8
    );
    assert!(config.observability.enabled);
    assert_eq!(config.observability.verbosity.to_string(), "summary");
}

#[test]
fn toml_overrides_document_sources_and_history() {
    let _guard = env_lock().lock().expect("env lock");
    let dir = tempdir().expect("tempdir");
    let explicit = dir.path().join("rarag.toml");

    write_config(
        &explicit,
        r#"
[history]
enabled = true
max_commits = 256

[[document_sources.rules]]
path_glob = "docs/specs/**"
kind = "spec"
parser = "markdown"
weight = 2.0

[[document_sources.rules]]
path_glob = "docs/tasks/tasks.csv"
kind = "tasks-registry"
parser = "csv"
weight = 1.3
"#,
    );

    unsafe {
        std::env::remove_var("RARAG_CONFIG");
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    let config = load_app_config(Some(&explicit)).expect("load explicit config");
    assert_eq!(config.history.max_commits, 256);
    assert_eq!(config.document_sources.rules.len(), 2);
    assert_eq!(config.document_sources.rules[0].path_glob, "docs/specs/**");
    assert_eq!(config.document_sources.rules[1].kind.as_str(), "tasks-registry");
    assert_eq!(config.document_sources.rules[1].parser.as_str(), "csv");
}
