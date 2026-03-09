use rarag_core::config::{
    AppConfig, CliConfig, DaemonConfig, EmbeddingProviderConfig, McpConfig,
    NeighborhoodWeightsConfig, ObservabilityConfig, ObservabilityVerbosity, LanceDbConfig,
    RerankWeightsConfig, RetrievalConfig, RuntimePaths, TantivyConfig, TursoConfig,
    VectorDistanceMetric,
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
        lancedb: LanceDbConfig {
            db_root: "/tmp/rarag/lancedb".into(),
            table: "rarag_chunks".into(),
            distance_metric: VectorDistanceMetric::Cosine,
        },
        embeddings: EmbeddingProviderConfig {
            base_url: "https://api.openai.com/v1".into(),
            endpoint_path: "/embeddings".into(),
            model: "text-embedding-3-small".into(),
            api_key_env: "EMBEDDING_API_KEY".into(),
            dimensions: 1_536,
        },
        retrieval: RetrievalConfig {
            rerank: RerankWeightsConfig::default(),
            neighborhood: NeighborhoodWeightsConfig::default(),
        },
        observability: ObservabilityConfig::default(),
        cli: Some(CliConfig { default_json: true }),
        daemon: Some(DaemonConfig {
            socket_path: "/run/user/1000/rarag/raragd.sock".into(),
        }),
        mcp: Some(McpConfig {
            socket_path: "/run/user/1000/rarag/raragd.sock".into(),
        }),
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
fn builds_default_app_config() {
    let config = AppConfig::default();

    assert_eq!(config.embeddings.base_url, "https://api.openai.com/v1");
    assert_eq!(config.embeddings.endpoint_path, "/embeddings");
    assert_eq!(config.embeddings.model, "text-embedding-3-small");
    assert_eq!(config.embeddings.dimensions, 1_536);
    assert_eq!(config.observability.enabled, false);
    assert_eq!(config.observability.verbosity, ObservabilityVerbosity::Off);
    assert_eq!(config.retrieval.rerank.find_examples_example_like, 0.8);
    assert_eq!(config.retrieval.neighborhood.same_file, 4.0);
    assert_eq!(config.lancedb.distance_metric, VectorDistanceMetric::Cosine);
}

#[test]
fn binary_sections_are_optional() {
    let config: AppConfig = serde_json::from_value(serde_json::json!({
        "runtime": {
            "socket_path": "/run/user/1000/rarag/raragd.sock",
            "state_root": "/tmp/rarag/state",
            "cache_root": "/tmp/rarag/cache"
        },
        "turso": {
            "database_url": "libsql://localhost",
            "auth_token_env": "TURSO_AUTH_TOKEN"
        },
        "tantivy": {
            "index_root": "/tmp/rarag/index"
        },
        "lancedb": {
            "db_root": "/tmp/rarag/lancedb",
            "table": "rarag_chunks",
            "distance_metric": "cosine"
        },
        "embeddings": {
            "base_url": "https://api.openai.com/v1",
            "endpoint_path": "/embeddings",
            "model": "text-embedding-3-small",
            "api_key_env": "OPENAI_API_KEY",
            "dimensions": 1536
        },
        "retrieval": {
            "rerank": {
                "find_examples_example_like": 0.8
            },
            "neighborhood": {
                "same_file": 4.0
            }
        },
        "observability": {
            "enabled": false,
            "verbosity": "off"
        }
    }))
    .expect("deserialize config without binary sections");

    assert!(config.cli.is_none());
    assert!(config.daemon.is_none());
    assert!(config.mcp.is_none());
}

#[test]
fn parses_rerank_and_observability_sections() {
    let config: AppConfig = serde_json::from_value(serde_json::json!({
        "runtime": {
            "socket_path": "/run/user/1000/rarag/raragd.sock",
            "state_root": "/tmp/rarag/state",
            "cache_root": "/tmp/rarag/cache"
        },
        "turso": {
            "database_url": "libsql://localhost",
            "auth_token_env": "TURSO_AUTH_TOKEN"
        },
        "tantivy": {
            "index_root": "/tmp/rarag/index"
        },
        "lancedb": {
            "db_root": "/tmp/rarag/lancedb",
            "table": "rarag_chunks",
            "distance_metric": "dot"
        },
        "embeddings": {
            "base_url": "https://api.openai.com/v1",
            "endpoint_path": "/embeddings",
            "model": "text-embedding-3-small",
            "api_key_env": "OPENAI_API_KEY",
            "dimensions": 1536
        },
        "retrieval": {
            "rerank": {
                "bounded_refactor_test_like": 1.1,
                "worktree_diff_blast_radius": 1.5
            },
            "neighborhood": {
                "same_file": 4.5,
                "semantic_impl_bounded_refactor": 9.0
            }
        },
        "observability": {
            "enabled": true,
            "verbosity": "detailed"
        }
    }))
    .expect("deserialize config with retrieval and observability");

    assert_eq!(config.retrieval.rerank.bounded_refactor_test_like, 1.1);
    assert_eq!(config.retrieval.rerank.worktree_diff_blast_radius, 1.5);
    assert_eq!(config.retrieval.neighborhood.same_file, 4.5);
    assert_eq!(config.lancedb.distance_metric, VectorDistanceMetric::Dot);
    assert_eq!(
        config.retrieval.neighborhood.semantic_impl_bounded_refactor,
        9.0
    );
    assert!(config.observability.enabled);
    assert_eq!(
        config.observability.verbosity,
        ObservabilityVerbosity::Detailed
    );
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
fn defaults_preserve_openai_embedding_endpoint_shape() {
    let config = AppConfig::default();

    assert_eq!(config.embeddings.base_url, "https://api.openai.com/v1");
    assert_eq!(config.embeddings.endpoint_path, "/embeddings");
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
