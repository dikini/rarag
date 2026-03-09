use std::path::PathBuf;

use rarag_core::chunking::RustChunker;
use rarag_core::embeddings::{EmbeddingProvider, OpenAiCompatibleEmbeddings};
use rarag_core::indexing::{ChunkIndexer, LanceDbPointStore, TantivyChunkStore};
use rarag_core::metadata::SnapshotStore;
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

#[test]
fn maps_chunk_to_tantivy_document() {
    let dir = tempdir().expect("tempdir");
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");
    let chunk = chunks
        .into_iter()
        .find(|chunk| chunk.symbol_path.as_deref() == Some("mini_repo::example_sum"))
        .expect("example_sum chunk");

    let store = TantivyChunkStore::open(dir.path()).expect("open tantivy index");
    store
        .index_chunks("snapshot-a", std::slice::from_ref(&chunk))
        .expect("index chunk");

    assert_eq!(store.document_count().expect("doc count"), 1);
    assert_eq!(
        store
            .search_exact_symbol("mini_repo::example_sum", 10)
            .expect("search exact symbol")
            .len(),
        1
    );
}

#[test]
fn builds_openai_compatible_embedding_request() {
    let provider = OpenAiCompatibleEmbeddings::new(
        "https://api.openai.com/v1",
        "/embeddings",
        "text-embedding-3-small",
        "EMBEDDING_API_KEY",
        1_536,
    )
    .expect("provider config");

    unsafe {
        std::env::set_var("EMBEDDING_API_KEY", "test-token");
    }
    let request = provider
        .build_request(&["alpha".to_string(), "beta".to_string()])
        .expect("build request");

    assert_eq!(
        request.url().as_str(),
        "https://api.openai.com/v1/embeddings"
    );
    assert_eq!(request.method().as_str(), "POST");
    assert!(request.headers().contains_key("authorization"));
}

#[test]
fn supports_configurable_embedding_endpoint_path() {
    let provider = OpenAiCompatibleEmbeddings::new(
        "https://proxy.example.invalid/openai",
        "v1/embeddings",
        "text-embedding-3-small",
        "EMBEDDING_API_KEY",
        1_536,
    )
    .expect("provider config");

    unsafe {
        std::env::set_var("EMBEDDING_API_KEY", "test-token");
    }
    let request = provider
        .build_request(&["alpha".to_string()])
        .expect("build request");

    assert_eq!(
        request.url().as_str(),
        "https://proxy.example.invalid/openai/v1/embeddings"
    );
}

#[test]
fn metadata_lexical_and_vector_counts_match() {
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
                "/repo/.worktrees/index-a",
                "abc123",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ))
            .await
            .expect("create snapshot");
        let tantivy = TantivyChunkStore::open(&tantivy_dir).expect("open tantivy");
        let lancedb = LanceDbPointStore::new_in_memory("memory://index-pipeline", "rarag_chunks", 4);
        let provider = StaticEmbeddingProvider { dimensions: 4 };
        let indexer = ChunkIndexer::new(&metadata, &tantivy, &lancedb, &provider);
        let chunks = RustChunker::new(80)
            .chunk_workspace(&fixture_root())
            .expect("chunk workspace");

        let counts = indexer
            .reindex_snapshot(&snapshot.id, &chunks)
            .await
            .expect("reindex snapshot");

        assert_eq!(counts.metadata_rows, counts.lexical_docs);
        assert_eq!(counts.lexical_docs, counts.vector_points);
    });
}

#[test]
fn reindexes_fixture_repository() {
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
                "/repo/.worktrees/index-a",
                "abc123",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ))
            .await
            .expect("create snapshot");
        let tantivy = TantivyChunkStore::open(&tantivy_dir).expect("open tantivy");
        let lancedb = LanceDbPointStore::new_in_memory("memory://index-pipeline", "rarag_chunks", 4);
        let provider = StaticEmbeddingProvider { dimensions: 4 };
        let indexer = ChunkIndexer::new(&metadata, &tantivy, &lancedb, &provider);
        let chunks = RustChunker::new(80)
            .chunk_workspace(&fixture_root())
            .expect("chunk workspace");

        let counts = indexer
            .reindex_snapshot(&snapshot.id, &chunks)
            .await
            .expect("reindex snapshot");
        let symbol_hits = indexer
            .tantivy_store()
            .search_exact_symbol("mini_repo::example_sum", 10)
            .expect("search indexed symbol");

        assert!(counts.metadata_rows >= 6);
        assert_eq!(symbol_hits.len(), 1);
        assert_eq!(
            indexer
                .lancedb_store()
                .point_count()
                .await
                .expect("point count"),
            counts.vector_points
        );
    });
}
