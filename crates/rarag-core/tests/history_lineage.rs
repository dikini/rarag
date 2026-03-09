use std::path::PathBuf;

use rarag_core::chunking::RustChunker;
use rarag_core::embeddings::EmbeddingProvider;
use rarag_core::history::{derive_lineage_edges, parse_name_status_rename_chain};
use rarag_core::indexing::{ChunkIndexer, LanceDbPointStore, TantivyChunkStore};
use rarag_core::metadata::{HistoryNodeRecord, SnapshotStore};
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
fn resolves_path_rename_chain() {
    let lines = vec![
        "R100\tsrc/old.rs\tsrc/new.rs".to_string(),
        "R090\tsrc/new.rs\tsrc/final.rs".to_string(),
    ];

    let chain = parse_name_status_rename_chain(&lines);
    assert_eq!(chain, vec!["src/old.rs", "src/new.rs", "src/final.rs"]);
}

#[test]
fn marks_heuristic_causal_edges_with_confidence() {
    let nodes = vec![
        HistoryNodeRecord::new(
            "c1",
            "snapshot-1",
            "commit",
            Some("abc123".to_string()),
            "fix reload behavior race",
        ),
        HistoryNodeRecord::new(
            "c2",
            "snapshot-1",
            "commit",
            Some("def456".to_string()),
            "follow-up docs update",
        ),
    ];
    let edges = derive_lineage_edges("snapshot-1", &nodes);
    assert!(edges.iter().any(|edge| {
        edge.edge_kind == "fixes" && edge.confidence > 0.0 && edge.confidence <= 1.0
    }));
}

#[test]
fn historical_objects_never_appear_in_present_state_results_without_selector() {
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
                "/repo/.worktrees/history-invariant",
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
        indexer
            .reindex_snapshot(&snapshot.id, &chunks)
            .await
            .expect("reindex snapshot");

        metadata
            .replace_history_nodes(
                &snapshot.id,
                &[HistoryNodeRecord::new(
                    "h1",
                    snapshot.id.clone(),
                    "commit",
                    Some("abc123".to_string()),
                    "history-only-sentinel-token",
                )],
            )
            .await
            .expect("store history");

        let retriever = RepositoryRetriever::new(&metadata, &tantivy, &lancedb, &provider);
        let response = retriever
            .retrieve(
                RetrievalRequest::new(
                    snapshot.id,
                    QueryMode::UnderstandSymbol,
                    "history-only-sentinel-token",
                )
                .with_limit(6),
            )
            .await
            .expect("retrieve");

        assert!(
            response
                .items
                .iter()
                .all(|item| item.chunk.chunk_kind != "HistoryNode")
        );
    });
}
