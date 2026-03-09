use std::path::{Path, PathBuf};

use rarag_core::chunking::RustChunker;
use rarag_core::config::{
    DocumentSourceKind, DocumentSourceParser, DocumentSourceRule, DocumentSourcesConfig,
    ObservabilityConfig, ObservabilityVerbosity, RetrievalConfig,
};
use rarag_core::config_loader::load_app_config;
use rarag_core::embeddings::EmbeddingProvider;
use rarag_core::indexing::{ChunkIndexer, LanceDbPointStore, TantivyChunkStore};
use rarag_core::metadata::SnapshotStore;
use rarag_core::retrieval::{QueryMode, RepositoryRetriever, RetrievalRequest};
use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;
use tokio::runtime::Runtime;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/mini_repo")
}

fn compat_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/compat_repo")
}

fn runtime() -> Runtime {
    Runtime::new().expect("tokio runtime")
}

struct StaticEmbeddingProvider {
    dimensions: usize,
}

impl EmbeddingProvider for StaticEmbeddingProvider {
    fn embed_texts(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Ok(inputs
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let mut vector = vec![0.0; self.dimensions];
                vector[index % self.dimensions] = 1.0;
                vector
            })
            .collect())
    }
}

async fn build_retriever_for(
    fixture_root: &Path,
    worktree_root: &str,
    max_body_bytes: usize,
) -> (
    String,
    tempfile::TempDir,
    SnapshotStore,
    TantivyChunkStore,
    LanceDbPointStore,
    StaticEmbeddingProvider,
) {
    let dir = tempdir().expect("tempdir");
    let metadata_path = dir.path().join("metadata.db");
    let tantivy_dir = dir.path().join("tantivy");
    let metadata = SnapshotStore::open_local(&metadata_path.display().to_string())
        .await
        .expect("open metadata store");
    let snapshot = metadata
        .create_or_get_snapshot(SnapshotKey::new(
            "/repo",
            worktree_root,
            "abc123",
            "x86_64-unknown-linux-gnu",
            ["default"],
            "dev",
        ))
        .await
        .expect("create snapshot");
    let tantivy = TantivyChunkStore::open(&tantivy_dir).expect("open tantivy");
    let lancedb = LanceDbPointStore::new_in_memory("memory://tests", "rarag_chunks", 4);
    let provider = StaticEmbeddingProvider { dimensions: 4 };
    let indexer = ChunkIndexer::new(&metadata, &tantivy, &lancedb, &provider);
    let chunks = RustChunker::new(max_body_bytes)
        .chunk_workspace(fixture_root)
        .expect("chunk workspace");
    indexer
        .reindex_snapshot(&snapshot.id, &chunks)
        .await
        .expect("reindex snapshot");

    (snapshot.id, dir, metadata, tantivy, lancedb, provider)
}

async fn build_retriever_for_with_chunker(
    fixture_root: &Path,
    worktree_root: &str,
    chunker: RustChunker,
) -> (
    String,
    tempfile::TempDir,
    SnapshotStore,
    TantivyChunkStore,
    LanceDbPointStore,
    StaticEmbeddingProvider,
) {
    let dir = tempdir().expect("tempdir");
    let metadata_path = dir.path().join("metadata.db");
    let tantivy_dir = dir.path().join("tantivy");
    let metadata = SnapshotStore::open_local(&metadata_path.display().to_string())
        .await
        .expect("open metadata store");
    let snapshot = metadata
        .create_or_get_snapshot(SnapshotKey::new(
            "/repo",
            worktree_root,
            "abc123",
            "x86_64-unknown-linux-gnu",
            ["default"],
            "dev",
        ))
        .await
        .expect("create snapshot");
    let tantivy = TantivyChunkStore::open(&tantivy_dir).expect("open tantivy");
    let lancedb = LanceDbPointStore::new_in_memory("memory://tests", "rarag_chunks", 4);
    let provider = StaticEmbeddingProvider { dimensions: 4 };
    let indexer = ChunkIndexer::new(&metadata, &tantivy, &lancedb, &provider);
    let chunks = chunker
        .chunk_workspace(fixture_root)
        .expect("chunk workspace");
    indexer
        .reindex_snapshot(&snapshot.id, &chunks)
        .await
        .expect("reindex snapshot");

    (snapshot.id, dir, metadata, tantivy, lancedb, provider)
}

async fn build_retriever() -> (
    String,
    tempfile::TempDir,
    SnapshotStore,
    TantivyChunkStore,
    LanceDbPointStore,
    StaticEmbeddingProvider,
) {
    build_retriever_for(&fixture_root(), "/repo/.worktrees/retrieval-a", 80).await
}

#[test]
fn prioritizes_exact_symbol_match() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::UnderstandSymbol,
                    "example_sum implementation",
                )
                .with_symbol_path("mini_repo::example_sum")
                .with_limit(6),
            )
            .await
            .expect("retrieve");

        let top = response.items.first().expect("at least one result");
        assert_eq!(
            top.chunk.symbol_path.as_deref(),
            Some("mini_repo::example_sum")
        );
        assert!(top.evidence.iter().any(|entry| entry == "exact_symbol"));
    });
}

#[test]
fn caps_neighborhood_size_by_mode() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(snapshot_id, QueryMode::UnderstandSymbol, "example_sum")
                    .with_symbol_path("mini_repo::example_sum")
                    .with_limit(20),
            )
            .await
            .expect("retrieve");

        assert!(response.items.len() <= QueryMode::UnderstandSymbol.neighborhood_cap());
    });
}

#[test]
fn results_never_cross_snapshot_boundary() {
    runtime().block_on(async {
        let dir = tempdir().expect("tempdir");
        let metadata_path = dir.path().join("metadata.db");
        let tantivy_dir = dir.path().join("tantivy");
        let metadata = SnapshotStore::open_local(&metadata_path.display().to_string())
            .await
            .expect("open metadata store");
        let snapshot_a = metadata
            .create_or_get_snapshot(SnapshotKey::new(
                "/repo",
                "/repo/.worktrees/retrieval-a",
                "abc123",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ))
            .await
            .expect("create snapshot a");
        let snapshot_b = metadata
            .create_or_get_snapshot(SnapshotKey::new(
                "/repo",
                "/repo/.worktrees/retrieval-b",
                "def456",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ))
            .await
            .expect("create snapshot b");
        let tantivy = TantivyChunkStore::open(&tantivy_dir).expect("open tantivy");
        let lancedb = LanceDbPointStore::new_in_memory("memory://tests", "rarag_chunks", 4);
        let provider = StaticEmbeddingProvider { dimensions: 4 };
        let indexer = ChunkIndexer::new(&metadata, &tantivy, &lancedb, &provider);
        let chunks = RustChunker::new(80)
            .chunk_workspace(&fixture_root())
            .expect("chunk workspace");
        indexer
            .reindex_snapshot(&snapshot_a.id, &chunks)
            .await
            .expect("reindex snapshot a");
        indexer
            .reindex_snapshot(&snapshot_b.id, &chunks)
            .await
            .expect("reindex snapshot b");

        let provider = StaticEmbeddingProvider { dimensions: 4 };
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(snapshot_a.id.clone(), QueryMode::BlastRadius, "example_sum")
                    .with_symbol_path("mini_repo::example_sum")
                    .with_limit(12),
            )
            .await
            .expect("retrieve");

        assert!(
            response
                .items
                .iter()
                .all(|item| item.snapshot_id == snapshot_a.id)
        );
    });
}

#[test]
fn bounded_refactor_returns_tests_and_references() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::BoundedRefactor,
                    "rename or refactor example_sum safely",
                )
                .with_symbol_path("mini_repo::example_sum")
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        assert!(
            response
                .items
                .iter()
                .any(|item| item.chunk.chunk_kind == "TestFunction")
        );
        assert!(response.items.iter().any(|item| {
            item.chunk.symbol_path.as_deref() == Some("mini_repo::example_sum")
                || item.evidence.iter().any(|entry| entry == "text_reference")
        }));
    });
}

#[test]
fn falls_back_to_lexical_bm25_when_symbol_path_is_missing() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(snapshot_id, QueryMode::FindExamples, "oversized_example")
                    .with_limit(6),
            )
            .await
            .expect("retrieve");

        assert!(response.items.iter().any(|item| {
            item.chunk.symbol_path.as_deref() == Some("mini_repo::oversized_example")
                && item.evidence.iter().any(|entry| entry == "lexical_bm25")
        }));
    });
}

#[test]
fn lexical_query_can_hit_docs_and_example_text() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-compat",
            120,
        )
        .await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::FindExamples,
                    "assert_eq!(doc_example_sum(2, 3), 5);",
                )
                .with_limit(6),
            )
            .await
            .expect("retrieve");

        assert!(response.items.iter().any(|item| {
            item.chunk.chunk_kind == "Doctest"
                && item.evidence.iter().any(|entry| entry == "lexical_bm25")
        }));
    });
}

#[test]
fn observation_capture_does_not_change_ranked_results() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever().await;
        let baseline = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider)
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id.clone(),
                    QueryMode::UnderstandSymbol,
                    "example_sum",
                )
                .with_symbol_path("mini_repo::example_sum")
                .with_limit(6),
            )
            .await
            .expect("baseline retrieve");

        let observed = RepositoryRetriever::new_with_settings(
            &metadata,
            &tantivy,
            &lancedb,
            &provider,
            &RetrievalConfig::default(),
            &ObservabilityConfig {
                enabled: true,
                verbosity: ObservabilityVerbosity::Summary,
            },
        )
        .retrieve(
            RetrievalRequest::new(snapshot_id, QueryMode::UnderstandSymbol, "example_sum")
                .with_symbol_path("mini_repo::example_sum")
                .with_limit(6),
        )
        .await
        .expect("observed retrieve");

        assert_eq!(baseline.items, observed.items);
        assert_eq!(baseline.warnings, observed.warnings);
    });
}

#[test]
fn detailed_observation_captures_candidate_features() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new_with_settings(
            &metadata,
            &tantivy,
            &lancedb,
            &provider,
            &RetrievalConfig::default(),
            &ObservabilityConfig {
                enabled: true,
                verbosity: ObservabilityVerbosity::Detailed,
            },
        );
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id.clone(),
                    QueryMode::BoundedRefactor,
                    "rename or refactor example_sum safely",
                )
                .with_symbol_path("mini_repo::example_sum")
                .with_limit(8),
            )
            .await
            .expect("retrieve");

        assert!(!response.items.is_empty());

        let observations = metadata
            .load_query_observations(&snapshot_id)
            .await
            .expect("load observations");
        let latest = observations.last().expect("observation row");
        let candidates = metadata
            .load_candidate_observations(&latest.observation_id)
            .await
            .expect("load candidate observations");

        assert!(!candidates.is_empty());
        assert!(candidates.iter().any(|candidate| candidate.returned));
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.final_score >= candidate.base_score)
        );
        assert!(candidates.iter().any(|candidate| {
            candidate
                .evidence
                .iter()
                .any(|entry| entry == "exact_symbol" || entry == "lexical_bm25")
        }));
    });
}

#[test]
fn prefers_normative_spec_over_plan_for_current_behavior() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-doc-priority",
            120,
        )
        .await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::BlastRadius,
                    "what is current reload behavior",
                )
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        let spec_rank = response.items.iter().position(|item| {
            item.chunk
                .file_path
                .ends_with("docs/specs/current-behavior.md")
        });
        let plan_rank = response
            .items
            .iter()
            .position(|item| item.chunk.file_path.ends_with("docs/plans/future-work.md"));

        assert!(spec_rank.is_some(), "expected spec evidence in result set");
        if let Some(plan_rank) = plan_rank {
            assert!(spec_rank < Some(plan_rank), "spec should rank above plan");
        }
    });
}

#[test]
fn returns_docs_code_and_tests_for_doc_constrained_change() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-doc-mixed",
            120,
        )
        .await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::BoundedRefactor,
                    "update doc_example_sum behavior docs and related tests",
                )
                .with_symbol_path("compat_repo::doc_example_sum")
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        assert!(response.items.iter().any(|item| {
            item.chunk.chunk_kind == "DocumentBlock" || item.chunk.chunk_kind == "TaskRow"
        }));
        assert!(response.items.iter().any(|item| {
            matches!(
                item.chunk.chunk_kind.as_str(),
                "Symbol" | "BodyRegion" | "CrateSummary"
            )
        }));
        assert!(
            response
                .items
                .iter()
                .any(|item| item.chunk.chunk_kind == "TestFunction")
                || response
                    .items
                    .iter()
                    .any(|item| item.chunk.chunk_kind == "Doctest")
        );
    });
}

#[test]
fn document_evidence_never_expands_to_unbounded_sibling_sections() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-doc-bounded",
            120,
        )
        .await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::UnderstandSymbol,
                    "reload behavior and current contract",
                )
                .with_limit(6),
            )
            .await
            .expect("retrieve");

        let spec_sections = response
            .items
            .iter()
            .filter(|item| {
                item.chunk
                    .file_path
                    .ends_with("docs/specs/current-behavior.md")
            })
            .count();
        assert!(
            spec_sections < response.items.len(),
            "expected bounded spec section expansion"
        );
    });
}

#[test]
fn toml_document_rank_weight_override_changes_mixed_document_ordering() {
    runtime().block_on(async {
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("rarag.toml");
        std::fs::write(
            &config_path,
            r#"
[[document_sources.rules]]
path_glob = "docs/specs/**"
kind = "spec"
parser = "markdown"
weight = 0.1

[[document_sources.rules]]
path_glob = "docs/plans/**"
kind = "plan"
parser = "markdown"
weight = 2.0

[[document_sources.rules]]
path_glob = "docs/ops/**"
kind = "ops"
parser = "markdown"
weight = 0.1

[[document_sources.rules]]
path_glob = "docs/integrations/**"
kind = "integrations"
parser = "markdown"
weight = 0.1

[[document_sources.rules]]
path_glob = "docs/templates/**"
kind = "documentation"
parser = "markdown"
weight = 0.1

[[document_sources.rules]]
path_glob = "CHANGELOG.md"
kind = "changelog"
parser = "markdown"
weight = 0.1

[[document_sources.rules]]
path_glob = "docs/tasks/tasks.csv"
kind = "tasks-registry"
parser = "csv"
weight = 0.1
"#,
        )
        .expect("write config");

        let config = load_app_config(Some(&config_path)).expect("load config");
        let chunker = RustChunker::new_with_document_sources(120, config.document_sources.clone());
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) =
            build_retriever_for_with_chunker(
                &compat_fixture_root(),
                "/repo/.worktrees/retrieval-doc-rank-override",
                chunker,
            )
            .await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::BlastRadius,
                    "reload behavior future work",
                )
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        let plan_rank = response
            .items
            .iter()
            .position(|item| item.chunk.file_path.ends_with("docs/plans/future-work.md"));
        let spec_rank = response.items.iter().position(|item| {
            item.chunk
                .file_path
                .ends_with("docs/specs/current-behavior.md")
        });
        assert!(plan_rank.is_some(), "expected plan evidence in result set");
        assert!(spec_rank.is_some(), "expected spec evidence in result set");
        assert!(
            plan_rank < spec_rank,
            "override should move plan above spec in mixed-doc ordering"
        );
    });
}

#[test]
fn toml_document_rank_weight_override_applies_without_reindex_markers() {
    runtime().block_on(async {
        let dir = tempdir().expect("tempdir");
        let metadata_path = dir.path().join("metadata.db");
        let tantivy_dir = dir.path().join("tantivy");
        let metadata = SnapshotStore::open_local(&metadata_path.display().to_string())
            .await
            .expect("open metadata store");
        let snapshot = metadata
            .create_or_get_snapshot(SnapshotKey::new(
                "/repo",
                "/repo/.worktrees/retrieval-doc-rank-no-reindex",
                "abc123",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ))
            .await
            .expect("create snapshot");
        let tantivy = TantivyChunkStore::open(&tantivy_dir).expect("open tantivy");
        let lancedb = LanceDbPointStore::new_in_memory("memory://tests", "rarag_chunks", 4);
        let provider = StaticEmbeddingProvider { dimensions: 4 };
        let indexer = ChunkIndexer::new(&metadata, &tantivy, &lancedb, &provider);
        let mut chunks = RustChunker::new(120)
            .chunk_workspace(&compat_fixture_root())
            .expect("chunk workspace");
        for chunk in &mut chunks {
            chunk
                .retrieval_markers
                .retain(|marker| !marker.starts_with("doc_rank_weight:"));
        }
        indexer
            .reindex_snapshot(&snapshot.id, &chunks)
            .await
            .expect("reindex snapshot");

        let document_sources = DocumentSourcesConfig {
            rules: vec![
                DocumentSourceRule::new(
                    "docs/specs/**",
                    DocumentSourceKind::Spec,
                    DocumentSourceParser::Markdown,
                    0.1,
                ),
                DocumentSourceRule::new(
                    "docs/plans/**",
                    DocumentSourceKind::Plan,
                    DocumentSourceParser::Markdown,
                    2.0,
                ),
                DocumentSourceRule::new(
                    "docs/ops/**",
                    DocumentSourceKind::Ops,
                    DocumentSourceParser::Markdown,
                    0.1,
                ),
                DocumentSourceRule::new(
                    "docs/integrations/**",
                    DocumentSourceKind::Integrations,
                    DocumentSourceParser::Markdown,
                    0.1,
                ),
                DocumentSourceRule::new(
                    "docs/templates/**",
                    DocumentSourceKind::Documentation,
                    DocumentSourceParser::Markdown,
                    0.1,
                ),
                DocumentSourceRule::new(
                    "CHANGELOG.md",
                    DocumentSourceKind::Changelog,
                    DocumentSourceParser::Markdown,
                    0.1,
                ),
                DocumentSourceRule::new(
                    "docs/tasks/tasks.csv",
                    DocumentSourceKind::TasksRegistry,
                    DocumentSourceParser::Csv,
                    0.1,
                ),
            ],
        };
        let retriever = RepositoryRetriever::new_with_full_settings(
            &metadata,
            &tantivy,
            &lancedb,
            &provider,
            &RetrievalConfig::default(),
            &ObservabilityConfig::default(),
            &document_sources,
        );
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot.id.clone(),
                    QueryMode::BlastRadius,
                    "reload behavior future work",
                )
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        let plan_rank = response
            .items
            .iter()
            .position(|item| item.chunk.file_path.ends_with("docs/plans/future-work.md"));
        let spec_rank = response.items.iter().position(|item| {
            item.chunk
                .file_path
                .ends_with("docs/specs/current-behavior.md")
        });
        assert!(plan_rank.is_some(), "expected plan evidence in result set");
        assert!(spec_rank.is_some(), "expected spec evidence in result set");
        assert!(
            plan_rank < spec_rank,
            "query-time doc source rules should apply even without indexed rank markers"
        );
    });
}

#[test]
fn history_selector_limits_results_to_requested_window() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-history-window",
            120,
        )
        .await;
        metadata
            .replace_history_nodes(
                &snapshot_id,
                &[
                    rarag_core::metadata::HistoryNodeRecord::new(
                        "h1",
                        snapshot_id.clone(),
                        "commit",
                        Some("a1".to_string()),
                        "reload behavior initial commit",
                    ),
                    rarag_core::metadata::HistoryNodeRecord::new(
                        "h2",
                        snapshot_id.clone(),
                        "commit",
                        Some("a2".to_string()),
                        "reload behavior docs adjustment",
                    ),
                    rarag_core::metadata::HistoryNodeRecord::new(
                        "h3",
                        snapshot_id.clone(),
                        "commit",
                        Some("a3".to_string()),
                        "reload behavior tests update",
                    ),
                ],
            )
            .await
            .expect("store history nodes");

        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(snapshot_id, QueryMode::BlastRadius, "reload behavior")
                    .with_history(true)
                    .with_history_max_nodes(2)
                    .with_limit(10),
            )
            .await
            .expect("retrieve");

        let history_count = response
            .items
            .iter()
            .filter(|item| item.chunk.chunk_kind == "HistoryNode")
            .count();
        assert!(history_count <= 2);
    });
}

#[test]
fn regression_archaeology_returns_changes_docs_and_tests() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-archaeology",
            120,
        )
        .await;
        metadata
            .replace_history_nodes(
                &snapshot_id,
                &[rarag_core::metadata::HistoryNodeRecord::new(
                    "h1",
                    snapshot_id.clone(),
                    "commit",
                    Some("a1".to_string()),
                    "changed reload behavior in daemon server",
                )],
            )
            .await
            .expect("store history nodes");

        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::BoundedRefactor,
                    "archaeology: changed doc_example_sum behavior in daemon server",
                )
                .with_symbol_path("compat_repo::doc_example_sum")
                .with_history(true)
                .with_history_max_nodes(4)
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        assert!(
            response
                .items
                .iter()
                .any(|item| item.chunk.chunk_kind == "HistoryNode")
        );
        assert!(
            response
                .items
                .iter()
                .any(|item| item.chunk.chunk_kind == "DocumentBlock")
        );
        assert!(
            response
                .items
                .iter()
                .any(|item| item.chunk.chunk_kind == "TestFunction")
                || response
                    .items
                    .iter()
                    .any(|item| item.chunk.chunk_kind == "Doctest")
        );
    });
}

#[test]
fn observation_trace_records_evidence_class_coverage() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, lancedb, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-eval-coverage",
            120,
        )
        .await;
        metadata
            .replace_history_nodes(
                &snapshot_id,
                &[rarag_core::metadata::HistoryNodeRecord::new(
                    "h1",
                    snapshot_id.clone(),
                    "commit",
                    Some("abc123".to_string()),
                    "changed doc_example_sum behavior in daemon server",
                )],
            )
            .await
            .expect("store history");

        let retriever = RepositoryRetriever::new_with_settings(
            &metadata,
            &tantivy,
            &lancedb,
            &provider,
            &RetrievalConfig::default(),
            &ObservabilityConfig {
                enabled: true,
                verbosity: ObservabilityVerbosity::Summary,
            },
        );
        let _ = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id.clone(),
                    QueryMode::BoundedRefactor,
                    "changed doc_example_sum behavior in daemon server",
                )
                .with_symbol_path("compat_repo::doc_example_sum")
                .with_history(true)
                .with_history_max_nodes(4)
                .with_limit(10),
            )
            .await
            .expect("retrieve");

        let observations = metadata
            .load_query_observations(&snapshot_id)
            .await
            .expect("load observations");
        let latest = observations.last().expect("observation");
        assert!(
            latest
                .evidence_class_coverage
                .iter()
                .any(|entry| entry == "code")
        );
        assert!(
            latest
                .evidence_class_coverage
                .iter()
                .any(|entry| entry == "document")
        );
        assert!(
            latest
                .evidence_class_coverage
                .iter()
                .any(|entry| entry == "history")
        );
    });
}
