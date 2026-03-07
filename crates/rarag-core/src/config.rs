use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub runtime: RuntimePaths,
    pub turso: TursoConfig,
    pub tantivy: TantivyConfig,
    pub qdrant: QdrantConfig,
    pub embeddings: EmbeddingProviderConfig,
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
