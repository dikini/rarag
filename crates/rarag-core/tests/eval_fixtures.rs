use std::path::PathBuf;

use rarag_core::chunking::RustChunker;
use rarag_core::config::{ObservabilityConfig, ObservabilityVerbosity, RetrievalConfig};
use rarag_core::embeddings::EmbeddingProvider;
use rarag_core::indexing::{ChunkIndexer, LanceDbPointStore, TantivyChunkStore};
use rarag_core::metadata::{HistoryNodeRecord, SnapshotStore};
use rarag_core::retrieval::{
    QueryMode, RepositoryRetriever, RetrievalRequest, load_eval_task_fixtures,
};
use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;
use tokio::runtime::Runtime;

fn compat_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/compat_repo")
}

fn eval_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/eval/tasks.json")
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
fn loads_task_with_ideal_acceptable_and_distractor_sets() {
    let tasks = load_eval_task_fixtures(&eval_fixture_path()).expect("load fixtures");
    assert_eq!(tasks.len(), 1);
    let task = &tasks[0];
    assert_eq!(task.task_id, "reload-archaeology");
    assert!(!task.ideal.is_empty());
    assert!(!task.acceptable.is_empty());
    assert!(!task.distractors.is_empty());
}

#[test]
fn revision_pinned_eval_task_replays_against_observation_store() {
    runtime().block_on(async {
        let tasks = load_eval_task_fixtures(&eval_fixture_path()).expect("load fixtures");
        let task = tasks.first().expect("fixture task");

        let dir = tempdir().expect("tempdir");
        let metadata_path = dir.path().join("metadata.db");
        let tantivy_dir = dir.path().join("tantivy");
        let metadata = SnapshotStore::open_local(&metadata_path.display().to_string())
            .await
            .expect("open metadata store");
        let snapshot = metadata
            .create_or_get_snapshot(SnapshotKey::new(
                "/repo",
                "/repo/.worktrees/eval-fixture",
                &task.revision,
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
        let chunks = RustChunker::new(120)
            .chunk_workspace(&compat_fixture_root())
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
        let mut request = RetrievalRequest::new(
            snapshot.id.clone(),
            parse_mode(&task.query_mode),
            task.query_text.clone(),
        )
        .with_eval_task_id(task.task_id.clone())
        .with_history(true)
        .with_history_max_nodes(4)
        .with_limit(10);
        if let Some(symbol_path) = task.symbol_path.as_deref() {
            request = request.with_symbol_path(symbol_path);
        }
        let _ = retriever.retrieve(request).await.expect("retrieve");

        let observations = metadata
            .load_query_observations(&snapshot.id)
            .await
            .expect("load observations");
        let latest = observations.last().expect("observation");
        assert_eq!(latest.eval_task_id.as_deref(), Some(task.task_id.as_str()));
        assert!(
            latest
                .evidence_class_coverage
                .iter()
                .any(|entry| entry == "history")
        );
    });
}

fn parse_mode(mode: &str) -> QueryMode {
    match mode {
        "understand-symbol" => QueryMode::UnderstandSymbol,
        "implement-adjacent" => QueryMode::ImplementAdjacent,
        "bounded-refactor" => QueryMode::BoundedRefactor,
        "find-examples" => QueryMode::FindExamples,
        "blast-radius" => QueryMode::BlastRadius,
        other => panic!("unsupported query mode {other}"),
    }
}
