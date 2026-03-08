use std::env;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub runtime: RuntimePaths,
    pub turso: TursoConfig,
    pub tantivy: TantivyConfig,
    pub qdrant: QdrantConfig,
    pub embeddings: EmbeddingProviderConfig,
    #[serde(default)]
    pub retrieval: RetrievalConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub cli: Option<CliConfig>,
    #[serde(default)]
    pub daemon: Option<DaemonConfig>,
    #[serde(default)]
    pub mcp: Option<McpConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let runtime_root = runtime_socket_root();
        let state_root = xdg_state_root();
        let cache_root = xdg_cache_root();

        Self {
            runtime: RuntimePaths::new(
                format!("{runtime_root}/{}", crate::workspace::default_socket_name()),
                format!("{state_root}/rarag"),
                format!("{cache_root}/rarag"),
            ),
            turso: TursoConfig {
                database_url: format!("file:{state_root}/rarag/metadata.db"),
                auth_token_env: "TURSO_AUTH_TOKEN".into(),
            },
            tantivy: TantivyConfig {
                index_root: format!("{cache_root}/rarag/tantivy"),
            },
            qdrant: QdrantConfig {
                endpoint: "http://127.0.0.1:6334".into(),
                collection: "rarag_chunks".into(),
            },
            embeddings: EmbeddingProviderConfig {
                base_url: "https://api.openai.com/v1".into(),
                endpoint_path: "/embeddings".into(),
                model: "text-embedding-3-small".into(),
                api_key_env: "OPENAI_API_KEY".into(),
                dimensions: 1_536,
            },
            retrieval: RetrievalConfig::default(),
            observability: ObservabilityConfig::default(),
            cli: None,
            daemon: Some(DaemonConfig {
                socket_path: format!("{runtime_root}/{}", crate::workspace::default_socket_name()),
            }),
            mcp: Some(McpConfig {
                socket_path: format!(
                    "{runtime_root}/{}",
                    crate::workspace::default_mcp_socket_name()
                ),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetrievalConfig {
    #[serde(default)]
    pub rerank: RerankWeightsConfig,
    #[serde(default)]
    pub neighborhood: NeighborhoodWeightsConfig,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            rerank: RerankWeightsConfig::default(),
            neighborhood: NeighborhoodWeightsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RerankWeightsConfig {
    pub understand_symbol_symbol: f32,
    pub implement_adjacent_body_region: f32,
    pub bounded_refactor_test_like: f32,
    pub bounded_refactor_other: f32,
    pub blast_radius_test_like: f32,
    pub blast_radius_other: f32,
    pub find_examples_example_like: f32,
    pub find_examples_other: f32,
    pub worktree_diff_understand_symbol: f32,
    pub worktree_diff_implement_adjacent: f32,
    pub worktree_diff_bounded_refactor: f32,
    pub worktree_diff_blast_radius: f32,
    pub worktree_diff_find_examples: f32,
}

impl Default for RerankWeightsConfig {
    fn default() -> Self {
        Self {
            understand_symbol_symbol: 0.6,
            implement_adjacent_body_region: 0.4,
            bounded_refactor_test_like: 0.6,
            bounded_refactor_other: 0.2,
            blast_radius_test_like: 0.6,
            blast_radius_other: 0.2,
            find_examples_example_like: 0.8,
            find_examples_other: 0.1,
            worktree_diff_understand_symbol: 0.4,
            worktree_diff_implement_adjacent: 0.8,
            worktree_diff_bounded_refactor: 1.2,
            worktree_diff_blast_radius: 1.2,
            worktree_diff_find_examples: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NeighborhoodWeightsConfig {
    pub exact_symbol: f32,
    pub same_file: f32,
    pub text_reference_understand_symbol: f32,
    pub text_reference_implement_adjacent: f32,
    pub text_reference_bounded_refactor: f32,
    pub text_reference_bounded_refactor_test_like: f32,
    pub text_reference_blast_radius: f32,
    pub text_reference_blast_radius_test_like: f32,
    pub text_reference_find_examples: f32,
    pub text_reference_find_examples_test_like: f32,
    pub test_neighbor_find_examples: f32,
    pub test_neighbor_bounded_refactor: f32,
    pub module_context_understand_symbol: f32,
    pub semantic_reference_understand_symbol: f32,
    pub semantic_reference_implement_adjacent: f32,
    pub semantic_reference_bounded_refactor: f32,
    pub semantic_reference_blast_radius: f32,
    pub semantic_reference_find_examples: f32,
    pub semantic_impl_understand_symbol: f32,
    pub semantic_impl_implement_adjacent: f32,
    pub semantic_impl_bounded_refactor: f32,
    pub semantic_impl_blast_radius: f32,
    pub semantic_impl_find_examples: f32,
    pub semantic_test_understand_symbol: f32,
    pub semantic_test_implement_adjacent: f32,
    pub semantic_test_bounded_refactor: f32,
    pub semantic_test_blast_radius: f32,
    pub semantic_test_find_examples: f32,
}

impl Default for NeighborhoodWeightsConfig {
    fn default() -> Self {
        Self {
            exact_symbol: 10.0,
            same_file: 4.0,
            text_reference_understand_symbol: 3.0,
            text_reference_implement_adjacent: 4.5,
            text_reference_bounded_refactor: 5.0,
            text_reference_bounded_refactor_test_like: 6.0,
            text_reference_blast_radius: 5.0,
            text_reference_blast_radius_test_like: 6.0,
            text_reference_find_examples: 5.5,
            text_reference_find_examples_test_like: 6.0,
            test_neighbor_find_examples: 3.5,
            test_neighbor_bounded_refactor: 3.5,
            module_context_understand_symbol: 2.5,
            semantic_reference_understand_symbol: 3.8,
            semantic_reference_implement_adjacent: 4.8,
            semantic_reference_bounded_refactor: 5.8,
            semantic_reference_blast_radius: 5.8,
            semantic_reference_find_examples: 5.2,
            semantic_impl_understand_symbol: 4.4,
            semantic_impl_implement_adjacent: 6.8,
            semantic_impl_bounded_refactor: 8.6,
            semantic_impl_blast_radius: 8.6,
            semantic_impl_find_examples: 4.0,
            semantic_test_understand_symbol: 3.6,
            semantic_test_implement_adjacent: 4.6,
            semantic_test_bounded_refactor: 8.2,
            semantic_test_blast_radius: 8.2,
            semantic_test_find_examples: 8.2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObservabilityConfig {
    pub enabled: bool,
    pub verbosity: ObservabilityVerbosity,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            verbosity: ObservabilityVerbosity::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservabilityVerbosity {
    Off,
    Summary,
    Detailed,
}

impl fmt::Display for ObservabilityVerbosity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => f.write_str("off"),
            Self::Summary => f.write_str("summary"),
            Self::Detailed => f.write_str("detailed"),
        }
    }
}

impl AppConfig {
    pub fn cli_default_json(&self) -> bool {
        self.cli
            .as_ref()
            .map(|config| config.default_json)
            .unwrap_or(false)
    }

    pub fn daemon_socket_path(&self) -> &str {
        self.daemon
            .as_ref()
            .map(|config| config.socket_path.as_str())
            .unwrap_or(self.runtime.socket_path.as_str())
    }

    pub fn mcp_socket_path(&self) -> &str {
        self.mcp
            .as_ref()
            .map(|config| config.socket_path.as_str())
            .unwrap_or(self.runtime.socket_path.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePaths {
    pub socket_path: String,
    pub state_root: String,
    pub cache_root: String,
}

impl RuntimePaths {
    pub fn new(
        socket_path: impl Into<String>,
        state_root: impl Into<String>,
        cache_root: impl Into<String>,
    ) -> Self {
        Self {
            socket_path: socket_path.into(),
            state_root: state_root.into(),
            cache_root: cache_root.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TursoConfig {
    pub database_url: String,
    pub auth_token_env: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TantivyConfig {
    pub index_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub endpoint: String,
    pub collection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingProviderConfig {
    pub base_url: String,
    pub endpoint_path: String,
    pub model: String,
    pub api_key_env: String,
    pub dimensions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliConfig {
    pub default_json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpConfig {
    pub socket_path: String,
}

fn runtime_socket_root() -> String {
    if let Ok(runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        return format!("{runtime_dir}/rarag");
    }

    let fallback = PathBuf::from(xdg_state_root()).join("rarag/run");
    ensure_private_runtime_root(&fallback);
    fallback.display().to_string()
}

fn xdg_state_root() -> String {
    env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        env::var("HOME")
            .map(|home| format!("{home}/.local/state"))
            .unwrap_or_else(|_| fallback_user_tmp_root("state"))
    })
}

fn xdg_cache_root() -> String {
    env::var("XDG_CACHE_HOME").unwrap_or_else(|_| {
        env::var("HOME")
            .map(|home| format!("{home}/.cache"))
            .unwrap_or_else(|_| fallback_user_tmp_root("cache"))
    })
}

fn fallback_user_tmp_root(kind: &str) -> String {
    #[cfg(unix)]
    let uid = rustix::process::getuid().as_raw();
    #[cfg(not(unix))]
    let uid = 0;
    format!("/tmp/rarag-uid-{uid}/{kind}")
}

fn ensure_private_runtime_root(path: &PathBuf) {
    if std::fs::create_dir_all(path).is_err() {
        return;
    }

    #[cfg(unix)]
    {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
    }
}

impl EmbeddingProviderConfig {
    pub fn validate(&self) -> Result<(), String> {
        let mut missing = Vec::new();

        if self.base_url.trim().is_empty() {
            missing.push("base_url");
        }
        if self.endpoint_path.trim().is_empty() {
            missing.push("endpoint_path");
        }
        if self.model.trim().is_empty() {
            missing.push("model");
        }
        if self.api_key_env.trim().is_empty() {
            missing.push("api_key_env");
        }
        if self.dimensions == 0 {
            missing.push("dimensions");
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "embedding provider config missing or invalid fields: {}",
                missing.join(", ")
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmbeddingProviderConfig, NeighborhoodWeightsConfig, ObservabilityConfig,
        ObservabilityVerbosity, RerankWeightsConfig, RetrievalConfig, RuntimePaths,
    };

    #[test]
    fn runtime_paths_new_keeps_input_order() {
        let paths = RuntimePaths::new("sock", "state", "cache");

        assert_eq!(paths.socket_path, "sock");
        assert_eq!(paths.state_root, "state");
        assert_eq!(paths.cache_root, "cache");
    }

    #[test]
    fn validate_rejects_missing_required_fields() {
        let err = EmbeddingProviderConfig {
            base_url: String::new(),
            endpoint_path: String::new(),
            model: String::new(),
            api_key_env: String::new(),
            dimensions: 0,
        }
        .validate()
        .expect_err("config should fail validation");

        assert!(err.contains("base_url"));
        assert!(err.contains("endpoint_path"));
        assert!(err.contains("model"));
        assert!(err.contains("api_key_env"));
        assert!(err.contains("dimensions"));
    }

    #[test]
    fn defaults_keep_observability_off() {
        let config = ObservabilityConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.verbosity, ObservabilityVerbosity::Off);
    }

    #[test]
    fn retrieval_defaults_preserve_existing_scores() {
        let config = RetrievalConfig::default();

        assert_eq!(config.rerank, RerankWeightsConfig::default());
        assert_eq!(config.neighborhood, NeighborhoodWeightsConfig::default());
        assert_eq!(config.rerank.find_examples_example_like, 0.8);
        assert_eq!(config.neighborhood.same_file, 4.0);
    }
}
