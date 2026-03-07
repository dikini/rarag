use std::env;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub runtime: RuntimePaths,
    pub turso: TursoConfig,
    pub tantivy: TantivyConfig,
    pub qdrant: QdrantConfig,
    pub embeddings: EmbeddingProviderConfig,
    #[serde(default)]
    pub cli: Option<CliConfig>,
    #[serde(default)]
    pub daemon: Option<DaemonConfig>,
    #[serde(default)]
    pub mcp: Option<McpConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let runtime_dir = xdg_runtime_dir();
        let state_root = xdg_state_root();
        let cache_root = xdg_cache_root();

        Self {
            runtime: RuntimePaths::new(
                format!("{runtime_dir}/rarag/raragd.sock"),
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
            cli: None,
            daemon: Some(DaemonConfig {
                socket_path: format!(
                    "{runtime_dir}/rarag/{}",
                    crate::workspace::default_socket_name()
                ),
            }),
            mcp: Some(McpConfig {
                socket_path: format!(
                    "{runtime_dir}/rarag/{}",
                    crate::workspace::default_mcp_socket_name()
                ),
            }),
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

fn xdg_runtime_dir() -> String {
    env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string())
}

fn xdg_state_root() -> String {
    env::var("XDG_STATE_HOME").unwrap_or_else(|_| {
        env::var("HOME")
            .map(|home| format!("{home}/.local/state"))
            .unwrap_or_else(|_| "/tmp".to_string())
    })
}

fn xdg_cache_root() -> String {
    env::var("XDG_CACHE_HOME").unwrap_or_else(|_| {
        env::var("HOME")
            .map(|home| format!("{home}/.cache"))
            .unwrap_or_else(|_| "/tmp".to_string())
    })
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
    use super::{EmbeddingProviderConfig, RuntimePaths};

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
}
