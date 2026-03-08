use std::path::{Path, PathBuf};

use rarag_core::chunking::RustChunker;
use rarag_core::config::{ObservabilityConfig, ObservabilityVerbosity, RetrievalConfig};
use rarag_core::embeddings::EmbeddingProvider;
use rarag_core::indexing::{ChunkIndexer, QdrantPointStore, TantivyChunkStore};
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
    QdrantPointStore,
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
    let qdrant = QdrantPointStore::new_in_memory("memory://tests", "rarag_chunks", 4);
    let provider = StaticEmbeddingProvider { dimensions: 4 };
    let indexer = ChunkIndexer::new(&metadata, &tantivy, &qdrant, &provider);
    let chunks = RustChunker::new(max_body_bytes)
        .chunk_workspace(fixture_root)
        .expect("chunk workspace");
    indexer
        .reindex_snapshot(&snapshot.id, &chunks)
        .await
        .expect("reindex snapshot");

    (snapshot.id, dir, metadata, tantivy, qdrant, provider)
}

async fn build_retriever() -> (
    String,
    tempfile::TempDir,
    SnapshotStore,
    TantivyChunkStore,
    QdrantPointStore,
    StaticEmbeddingProvider,
) {
    build_retriever_for(&fixture_root(), "/repo/.worktrees/retrieval-a", 80).await
}

#[test]
fn prioritizes_exact_symbol_match() {
    runtime().block_on(async {
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider);
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
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::UnderstandSymbol,
                    "example_sum",
                )
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
        let qdrant = QdrantPointStore::new_in_memory("memory://tests", "rarag_chunks", 4);
        let provider = StaticEmbeddingProvider { dimensions: 4 };
        let indexer = ChunkIndexer::new(&metadata, &tantivy, &qdrant, &provider);
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
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_a.id.clone(),
                    QueryMode::BlastRadius,
                    "example_sum",
                )
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
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider);
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
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::FindExamples,
                    "oversized_example",
                )
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
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever_for(
            &compat_fixture_root(),
            "/repo/.worktrees/retrieval-compat",
            120,
        )
        .await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider);
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
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever().await;
        let baseline = RepositoryRetriever::new(&metadata, &tantivy, &qdrant, &provider)
            .retrieve(
                RetrievalRequest::new(snapshot_id.clone(), QueryMode::UnderstandSymbol, "example_sum")
                    .with_symbol_path("mini_repo::example_sum")
                    .with_limit(6),
            )
            .await
            .expect("baseline retrieve");

        let observed = RepositoryRetriever::new_with_settings(
            &metadata,
            &tantivy,
            &qdrant,
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
        let (snapshot_id, _dir, metadata, tantivy, qdrant, provider) = build_retriever().await;
        let retriever = RepositoryRetriever::new_with_settings(
            &metadata,
            &tantivy,
            &qdrant,
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
        assert!(candidates.iter().all(|candidate| candidate.final_score >= candidate.base_score));
        assert!(candidates.iter().any(|candidate| {
            candidate
                .evidence
                .iter()
                .any(|entry| entry == "exact_symbol" || entry == "lexical_bm25")
        }));
    });
}
