use std::env;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::{AppConfig, CliConfig, DaemonConfig, McpConfig};

pub fn load_app_config(explicit_path: Option<&Path>) -> Result<AppConfig, String> {
    load_app_config_with_source(explicit_path).map(|loaded| loaded.config)
}

#[derive(Debug, Clone)]
pub struct LoadedAppConfig {
    pub config: AppConfig,
    pub source_path: Option<PathBuf>,
}

pub fn load_app_config_with_source(
    explicit_path: Option<&Path>,
) -> Result<LoadedAppConfig, String> {
    let mut config = AppConfig::default();
    let source_path = resolve_config_path(explicit_path);

    if let Some(path) = source_path.as_ref() {
        let body = std::fs::read_to_string(&path)
            .map_err(|err| format!("failed to read config {}: {err}", path.display()))?;
        let overrides: PartialAppConfig = toml::from_str(&body)
            .map_err(|err| format!("failed to parse config {}: {err}", path.display()))?;
        apply_overrides(&mut config, overrides);
    }

    Ok(LoadedAppConfig {
        config,
        source_path,
    })
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

    if let Some(lancedb) = overrides.lancedb {
        if let Some(db_root) = lancedb.db_root {
            config.lancedb.db_root = db_root;
        }
        if let Some(table) = lancedb.table {
            config.lancedb.table = table;
        }
        if let Some(distance_metric) = lancedb.distance_metric {
            config.lancedb.distance_metric = distance_metric;
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

    if let Some(retrieval) = overrides.retrieval {
        if let Some(rerank) = retrieval.rerank {
            let resolved = &mut config.retrieval.rerank;
            if let Some(value) = rerank.understand_symbol_symbol {
                resolved.understand_symbol_symbol = value;
            }
            if let Some(value) = rerank.implement_adjacent_body_region {
                resolved.implement_adjacent_body_region = value;
            }
            if let Some(value) = rerank.bounded_refactor_test_like {
                resolved.bounded_refactor_test_like = value;
            }
            if let Some(value) = rerank.bounded_refactor_other {
                resolved.bounded_refactor_other = value;
            }
            if let Some(value) = rerank.blast_radius_test_like {
                resolved.blast_radius_test_like = value;
            }
            if let Some(value) = rerank.blast_radius_other {
                resolved.blast_radius_other = value;
            }
            if let Some(value) = rerank.find_examples_example_like {
                resolved.find_examples_example_like = value;
            }
            if let Some(value) = rerank.find_examples_other {
                resolved.find_examples_other = value;
            }
            if let Some(value) = rerank.worktree_diff_understand_symbol {
                resolved.worktree_diff_understand_symbol = value;
            }
            if let Some(value) = rerank.worktree_diff_implement_adjacent {
                resolved.worktree_diff_implement_adjacent = value;
            }
            if let Some(value) = rerank.worktree_diff_bounded_refactor {
                resolved.worktree_diff_bounded_refactor = value;
            }
            if let Some(value) = rerank.worktree_diff_blast_radius {
                resolved.worktree_diff_blast_radius = value;
            }
            if let Some(value) = rerank.worktree_diff_find_examples {
                resolved.worktree_diff_find_examples = value;
            }
        }

        if let Some(neighborhood) = retrieval.neighborhood {
            let resolved = &mut config.retrieval.neighborhood;
            if let Some(value) = neighborhood.exact_symbol {
                resolved.exact_symbol = value;
            }
            if let Some(value) = neighborhood.same_file {
                resolved.same_file = value;
            }
            if let Some(value) = neighborhood.text_reference_understand_symbol {
                resolved.text_reference_understand_symbol = value;
            }
            if let Some(value) = neighborhood.text_reference_implement_adjacent {
                resolved.text_reference_implement_adjacent = value;
            }
            if let Some(value) = neighborhood.text_reference_bounded_refactor {
                resolved.text_reference_bounded_refactor = value;
            }
            if let Some(value) = neighborhood.text_reference_bounded_refactor_test_like {
                resolved.text_reference_bounded_refactor_test_like = value;
            }
            if let Some(value) = neighborhood.text_reference_blast_radius {
                resolved.text_reference_blast_radius = value;
            }
            if let Some(value) = neighborhood.text_reference_blast_radius_test_like {
                resolved.text_reference_blast_radius_test_like = value;
            }
            if let Some(value) = neighborhood.text_reference_find_examples {
                resolved.text_reference_find_examples = value;
            }
            if let Some(value) = neighborhood.text_reference_find_examples_test_like {
                resolved.text_reference_find_examples_test_like = value;
            }
            if let Some(value) = neighborhood.test_neighbor_find_examples {
                resolved.test_neighbor_find_examples = value;
            }
            if let Some(value) = neighborhood.test_neighbor_bounded_refactor {
                resolved.test_neighbor_bounded_refactor = value;
            }
            if let Some(value) = neighborhood.module_context_understand_symbol {
                resolved.module_context_understand_symbol = value;
            }
            if let Some(value) = neighborhood.semantic_reference_understand_symbol {
                resolved.semantic_reference_understand_symbol = value;
            }
            if let Some(value) = neighborhood.semantic_reference_implement_adjacent {
                resolved.semantic_reference_implement_adjacent = value;
            }
            if let Some(value) = neighborhood.semantic_reference_bounded_refactor {
                resolved.semantic_reference_bounded_refactor = value;
            }
            if let Some(value) = neighborhood.semantic_reference_blast_radius {
                resolved.semantic_reference_blast_radius = value;
            }
            if let Some(value) = neighborhood.semantic_reference_find_examples {
                resolved.semantic_reference_find_examples = value;
            }
            if let Some(value) = neighborhood.semantic_impl_understand_symbol {
                resolved.semantic_impl_understand_symbol = value;
            }
            if let Some(value) = neighborhood.semantic_impl_implement_adjacent {
                resolved.semantic_impl_implement_adjacent = value;
            }
            if let Some(value) = neighborhood.semantic_impl_bounded_refactor {
                resolved.semantic_impl_bounded_refactor = value;
            }
            if let Some(value) = neighborhood.semantic_impl_blast_radius {
                resolved.semantic_impl_blast_radius = value;
            }
            if let Some(value) = neighborhood.semantic_impl_find_examples {
                resolved.semantic_impl_find_examples = value;
            }
            if let Some(value) = neighborhood.semantic_test_understand_symbol {
                resolved.semantic_test_understand_symbol = value;
            }
            if let Some(value) = neighborhood.semantic_test_implement_adjacent {
                resolved.semantic_test_implement_adjacent = value;
            }
            if let Some(value) = neighborhood.semantic_test_bounded_refactor {
                resolved.semantic_test_bounded_refactor = value;
            }
            if let Some(value) = neighborhood.semantic_test_blast_radius {
                resolved.semantic_test_blast_radius = value;
            }
            if let Some(value) = neighborhood.semantic_test_find_examples {
                resolved.semantic_test_find_examples = value;
            }
        }
    }

    if let Some(observability) = overrides.observability {
        if let Some(enabled) = observability.enabled {
            config.observability.enabled = enabled;
        }
        if let Some(verbosity) = observability.verbosity {
            config.observability.verbosity = verbosity;
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
    lancedb: Option<PartialLanceDbConfig>,
    embeddings: Option<PartialEmbeddingProviderConfig>,
    retrieval: Option<PartialRetrievalConfig>,
    observability: Option<PartialObservabilityConfig>,
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
struct PartialLanceDbConfig {
    db_root: Option<String>,
    table: Option<String>,
    distance_metric: Option<crate::config::VectorDistanceMetric>,
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
struct PartialRetrievalConfig {
    rerank: Option<PartialRerankWeightsConfig>,
    neighborhood: Option<PartialNeighborhoodWeightsConfig>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialRerankWeightsConfig {
    understand_symbol_symbol: Option<f32>,
    implement_adjacent_body_region: Option<f32>,
    bounded_refactor_test_like: Option<f32>,
    bounded_refactor_other: Option<f32>,
    blast_radius_test_like: Option<f32>,
    blast_radius_other: Option<f32>,
    find_examples_example_like: Option<f32>,
    find_examples_other: Option<f32>,
    worktree_diff_understand_symbol: Option<f32>,
    worktree_diff_implement_adjacent: Option<f32>,
    worktree_diff_bounded_refactor: Option<f32>,
    worktree_diff_blast_radius: Option<f32>,
    worktree_diff_find_examples: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialNeighborhoodWeightsConfig {
    exact_symbol: Option<f32>,
    same_file: Option<f32>,
    text_reference_understand_symbol: Option<f32>,
    text_reference_implement_adjacent: Option<f32>,
    text_reference_bounded_refactor: Option<f32>,
    text_reference_bounded_refactor_test_like: Option<f32>,
    text_reference_blast_radius: Option<f32>,
    text_reference_blast_radius_test_like: Option<f32>,
    text_reference_find_examples: Option<f32>,
    text_reference_find_examples_test_like: Option<f32>,
    test_neighbor_find_examples: Option<f32>,
    test_neighbor_bounded_refactor: Option<f32>,
    module_context_understand_symbol: Option<f32>,
    semantic_reference_understand_symbol: Option<f32>,
    semantic_reference_implement_adjacent: Option<f32>,
    semantic_reference_bounded_refactor: Option<f32>,
    semantic_reference_blast_radius: Option<f32>,
    semantic_reference_find_examples: Option<f32>,
    semantic_impl_understand_symbol: Option<f32>,
    semantic_impl_implement_adjacent: Option<f32>,
    semantic_impl_bounded_refactor: Option<f32>,
    semantic_impl_blast_radius: Option<f32>,
    semantic_impl_find_examples: Option<f32>,
    semantic_test_understand_symbol: Option<f32>,
    semantic_test_implement_adjacent: Option<f32>,
    semantic_test_bounded_refactor: Option<f32>,
    semantic_test_blast_radius: Option<f32>,
    semantic_test_find_examples: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
struct PartialObservabilityConfig {
    enabled: Option<bool>,
    verbosity: Option<crate::config::ObservabilityVerbosity>,
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
