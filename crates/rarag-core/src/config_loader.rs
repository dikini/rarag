use std::env;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::{AppConfig, CliConfig, DaemonConfig, McpConfig};

pub fn load_app_config(explicit_path: Option<&Path>) -> Result<AppConfig, String> {
    let mut config = AppConfig::default();

    if let Some(path) = resolve_config_path(explicit_path) {
        let body = std::fs::read_to_string(&path)
            .map_err(|err| format!("failed to read config {}: {err}", path.display()))?;
        let overrides: PartialAppConfig = toml::from_str(&body)
            .map_err(|err| format!("failed to parse config {}: {err}", path.display()))?;
        apply_overrides(&mut config, overrides);
    }

    Ok(config)
}

fn resolve_config_path(explicit_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit_path {
        return Some(path.to_path_buf());
    }

    if let Ok(path) = env::var("RARAG_CONFIG") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    if let Ok(xdg_root) = env::var("XDG_CONFIG_HOME") {
        let candidate = PathBuf::from(xdg_root).join("rarag/rarag.toml");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    if let Ok(home) = env::var("HOME") {
        let candidate = PathBuf::from(home).join(".config/rarag/rarag.toml");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn apply_overrides(config: &mut AppConfig, overrides: PartialAppConfig) {
    if let Some(runtime) = overrides.runtime {
        if let Some(socket_path) = runtime.socket_path {
            config.runtime.socket_path = socket_path.clone();
            config.daemon = Some(DaemonConfig {
                socket_path: socket_path.clone(),
            });
            config.mcp = Some(McpConfig {
                socket_path: derive_companion_mcp_socket(&socket_path),
            });
        }
        if let Some(state_root) = runtime.state_root {
            config.runtime.state_root = state_root;
        }
        if let Some(cache_root) = runtime.cache_root {
            config.runtime.cache_root = cache_root;
        }
    }

    if let Some(turso) = overrides.turso {
        if let Some(database_url) = turso.database_url {
            config.turso.database_url = database_url;
        }
        if let Some(auth_token_env) = turso.auth_token_env {
            config.turso.auth_token_env = auth_token_env;
        }
    }

    if let Some(tantivy) = overrides.tantivy
        && let Some(index_root) = tantivy.index_root
    {
        config.tantivy.index_root = index_root;
    }

    if let Some(qdrant) = overrides.qdrant {
        if let Some(endpoint) = qdrant.endpoint {
            config.qdrant.endpoint = endpoint;
        }
        if let Some(collection) = qdrant.collection {
            config.qdrant.collection = collection;
        }
    }

    if let Some(embeddings) = overrides.embeddings {
        if let Some(base_url) = embeddings.base_url {
            config.embeddings.base_url = base_url;
        }
        if let Some(endpoint_path) = embeddings.endpoint_path {
            config.embeddings.endpoint_path = endpoint_path;
        }
        if let Some(model) = embeddings.model {
            config.embeddings.model = model;
        }
        if let Some(api_key_env) = embeddings.api_key_env {
            config.embeddings.api_key_env = api_key_env;
        }
        if let Some(dimensions) = embeddings.dimensions {
            config.embeddings.dimensions = dimensions;
        }
    }

    if let Some(cli) = overrides.cli {
        let mut resolved = config.cli.take().unwrap_or(CliConfig {
            default_json: false,
        });
        if let Some(default_json) = cli.default_json {
            resolved.default_json = default_json;
        }
        config.cli = Some(resolved);
    }

    if let Some(daemon) = overrides.daemon {
        let mut resolved = config.daemon.take().unwrap_or(DaemonConfig {
            socket_path: config.runtime.socket_path.clone(),
        });
        if let Some(socket_path) = daemon.socket_path {
            resolved.socket_path = socket_path;
        }
        config.daemon = Some(resolved);
    }

    if let Some(mcp) = overrides.mcp {
        let mut resolved = config.mcp.take().unwrap_or(McpConfig {
            socket_path: config.runtime.socket_path.clone(),
        });
        if let Some(socket_path) = mcp.socket_path {
            resolved.socket_path = socket_path;
        }
        config.mcp = Some(resolved);
    }
}

#[derive(Debug, Deserialize, Default)]
struct PartialAppConfig {
    runtime: Option<PartialRuntimePaths>,
    turso: Option<PartialTursoConfig>,
    tantivy: Option<PartialTantivyConfig>,
    qdrant: Option<PartialQdrantConfig>,
    embeddings: Option<PartialEmbeddingProviderConfig>,
    cli: Option<PartialCliConfig>,
    daemon: Option<PartialDaemonConfig>,
    mcp: Option<PartialMcpConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialRuntimePaths {
    socket_path: Option<String>,
    state_root: Option<String>,
    cache_root: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialTursoConfig {
    database_url: Option<String>,
    auth_token_env: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialTantivyConfig {
    index_root: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialQdrantConfig {
    endpoint: Option<String>,
    collection: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialEmbeddingProviderConfig {
    base_url: Option<String>,
    endpoint_path: Option<String>,
    model: Option<String>,
    api_key_env: Option<String>,
    dimensions: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialCliConfig {
    default_json: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialDaemonConfig {
    socket_path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialMcpConfig {
    socket_path: Option<String>,
}

fn derive_companion_mcp_socket(socket_path: &str) -> String {
    if let Some(prefix) = socket_path.strip_suffix(".sock") {
        return format!("{prefix}-mcp.sock");
    }
    format!("{socket_path}-mcp")
}
