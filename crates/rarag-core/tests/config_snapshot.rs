use rarag_core::config::{
    AppConfig, EmbeddingProviderConfig, QdrantConfig, RuntimePaths, TantivyConfig, TursoConfig,
};
use rarag_core::snapshot::SnapshotKey;

fn sample_config() -> AppConfig {
    AppConfig {
        runtime: RuntimePaths::new(
            "/run/user/1000/rarag/raragd.sock",
            "/tmp/rarag/state",
            "/tmp/rarag/cache",
        ),
        turso: TursoConfig {
            database_url: "libsql://localhost".into(),
            auth_token_env: "TURSO_AUTH_TOKEN".into(),
        },
        tantivy: TantivyConfig {
            index_root: "/tmp/rarag/index".into(),
        },
        qdrant: QdrantConfig {
            endpoint: "http://127.0.0.1:6334".into(),
            collection: "rarag_chunks".into(),
        },
        embeddings: EmbeddingProviderConfig {
            base_url: "https://api.openai.com/v1".into(),
            endpoint_path: "/embeddings".into(),
            model: "text-embedding-3-small".into(),
            api_key_env: "EMBEDDING_API_KEY".into(),
            dimensions: 1_536,
        },
    }
}

#[test]
fn parses_runtime_paths() {
    let config = sample_config();

    assert_eq!(
        config.runtime.socket_path,
        "/run/user/1000/rarag/raragd.sock"
    );
    assert_eq!(config.runtime.state_root, "/tmp/rarag/state");
    assert_eq!(config.runtime.cache_root, "/tmp/rarag/cache");
}

#[test]
fn rejects_incomplete_embedding_config() {
    let err = EmbeddingProviderConfig {
        base_url: String::new(),
        endpoint_path: String::new(),
        model: "text-embedding-3-small".into(),
        api_key_env: String::new(),
        dimensions: 0,
    }
    .validate()
    .expect_err("embedding config should fail validation");

    assert!(err.contains("base_url"), "unexpected error: {err}");
    assert!(err.contains("endpoint_path"), "unexpected error: {err}");
    assert!(err.contains("api_key_env"), "unexpected error: {err}");
    assert!(err.contains("dimensions"), "unexpected error: {err}");
}

#[test]
fn snapshot_identity_changes_when_worktree_changes() {
    let left = SnapshotKey::new(
        "/repo",
        "/repo/.worktrees/alpha",
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["default", "sqlite"],
        "dev",
    );
    let right = SnapshotKey::new(
        "/repo",
        "/repo/.worktrees/beta",
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["default", "sqlite"],
        "dev",
    );

    assert_ne!(left.id(), right.id());
}

#[test]
fn snapshot_key_roundtrips_to_json() {
    let key = SnapshotKey::new(
        "/repo",
        "/repo/.worktrees/alpha",
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["sqlite", "default"],
        "dev",
    );

    let encoded = serde_json::to_string(&key).expect("serialize snapshot key");
    let decoded: SnapshotKey = serde_json::from_str(&encoded).expect("deserialize snapshot key");

    assert_eq!(decoded, key);
    assert_eq!(decoded.feature_set, vec!["default", "sqlite"]);
}
