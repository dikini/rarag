use std::path::PathBuf;

use rarag_core::chunking::RustChunker;
use rarag_core::embeddings::EmbeddingProvider;
use rarag_core::indexing::{ChunkIndexer, LanceDbPointStore, TantivyChunkStore};
use rarag_core::metadata::SnapshotStore;
use rarag_core::retrieval::{QueryMode, RepositoryRetriever, RetrievalRequest};
use rarag_core::semantic::{RustAnalyzerEnricher, SemanticEdgeKind};
use rarag_core::snapshot::SnapshotKey;
use rarag_core::worktree::WorktreeChanges;
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

async fn build_retriever() -> (
    String,
    tempfile::TempDir,
    PathBuf,
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
            "/repo/.worktrees/semantic-a",
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
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");
    let enricher = RustAnalyzerEnricher::heuristic();
    let enrichment = enricher
        .enrich_chunks(&fixture_root(), &chunks)
        .expect("semantic enrichment");
    indexer
        .reindex_snapshot_with_semantics(&snapshot.id, &chunks, &enrichment.edges)
        .await
        .expect("reindex snapshot");

    (
        snapshot.id,
        dir,
        fixture_root(),
        metadata,
        tantivy,
        lancedb,
        provider,
    )
}

#[test]
fn maps_reference_results_to_chunk_edges() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");
    let enricher = RustAnalyzerEnricher::heuristic();
    let enrichment = enricher
        .enrich_chunks(&fixture_root(), &chunks)
        .expect("semantic enrichment");

    assert!(enrichment.edges.iter().any(|edge| {
        edge.kind == SemanticEdgeKind::Reference
            && edge.to_symbol_path.as_deref() == Some("mini_repo::example_sum")
            && edge.from_symbol_path.as_deref() == Some("mini_repo::tests::example_sum_smoke")
    }));
}

#[test]
fn falls_back_when_analysis_unavailable() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");
    let enricher = RustAnalyzerEnricher::unavailable("fixture rust-analyzer disabled");
    let enrichment = enricher
        .enrich_chunks(&fixture_root(), &chunks)
        .expect("fallback enrichment");

    assert!(enrichment.edges.is_empty());
    assert!(
        enrichment
            .warnings
            .iter()
            .any(|warning| warning.contains("fixture rust-analyzer disabled"))
    );
}

#[test]
fn enrichment_never_rewrites_chunk_source_spans() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");
    let before: Vec<_> = chunks
        .iter()
        .map(|chunk| (chunk.id.clone(), chunk.span.start_byte, chunk.span.end_byte))
        .collect();
    let enricher = RustAnalyzerEnricher::heuristic();
    let _ = enricher
        .enrich_chunks(&fixture_root(), &chunks)
        .expect("semantic enrichment");
    let after: Vec<_> = chunks
        .iter()
        .map(|chunk| (chunk.id.clone(), chunk.span.start_byte, chunk.span.end_byte))
        .collect();

    assert_eq!(before, after);
}

#[test]
fn bounded_refactor_uses_impl_and_test_edges() {
    runtime().block_on(async {
        let (snapshot_id, _dir, fixture_root, metadata, tantivy, lancedb, provider) =
            build_retriever().await;
        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let changed = WorktreeChanges::from_paths([fixture_root.join("src/lib.rs")]);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_id,
                    QueryMode::BoundedRefactor,
                    "refactor Data safely",
                )
                .with_symbol_path("mini_repo::Data")
                .with_limit(8)
                .with_worktree_changes(changed),
            )
            .await
            .expect("retrieve");

        assert!(response.items.iter().any(|item| {
            item.evidence.iter().any(|entry| entry == "semantic_impl")
                && item.chunk.symbol_path.as_deref() == Some("mini_repo::impl::Data")
        }));
        assert!(response.items.iter().any(|item| {
            item.evidence.iter().any(|entry| entry == "semantic_test")
                && item.chunk.chunk_kind == "TestFunction"
        }));
        assert!(
            response
                .items
                .iter()
                .any(|item| item.evidence.iter().any(|entry| entry == "worktree_diff"))
        );
    });
}

#[test]
fn mixed_code_and_doc_evidence_preserves_snapshot_boundary() {
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
                "/repo/.worktrees/semantic-doc-a",
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
                "/repo/.worktrees/semantic-doc-b",
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
        let chunks = RustChunker::new(120)
            .chunk_workspace(&compat_fixture_root())
            .expect("chunk workspace");
        indexer
            .reindex_snapshot(&snapshot_a.id, &chunks)
            .await
            .expect("reindex snapshot a");
        indexer
            .reindex_snapshot(&snapshot_b.id, &chunks)
            .await
            .expect("reindex snapshot b");

        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot_a.id.clone(),
                    QueryMode::BoundedRefactor,
                    "reload behavior docs and tests",
                )
                .with_limit(8),
            )
            .await
            .expect("retrieve");

        assert!(!response.items.is_empty());
        assert!(
            response
                .items
                .iter()
                .all(|item| item.snapshot_id == snapshot_a.id)
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
                .any(|item| item.chunk.chunk_kind == "Symbol")
        );
    });
}
