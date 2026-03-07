use std::path::PathBuf;

use rarag_core::chunking::RustChunker;
use rarag_core::metadata::SnapshotStore;
use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;
use tokio::runtime::Runtime;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/compat_repo")
}

fn runtime() -> Runtime {
    Runtime::new().expect("tokio runtime")
}

#[test]
fn indexes_examples_and_doctests_as_first_class_chunks() {
    let chunks = RustChunker::new(120)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    assert!(chunks.iter().any(|chunk| {
        chunk.file_path.ends_with("examples/demo.rs") && chunk.text.contains("doc_example_sum")
    }));
    assert!(chunks.iter().any(|chunk| {
        chunk.file_path.ends_with("tests/integration.rs")
            && chunk.text.contains("integration_sum_smoke")
    }));
    assert!(chunks.iter().any(|chunk| {
        chunk.symbol_path.as_deref() == Some("compat_repo::doc_example_sum")
            && chunk.text.contains("assert_eq!(doc_example_sum(2, 3), 5);")
    }));
}

#[test]
fn metadata_roundtrips_example_and_doctest_chunks() {
    runtime().block_on(async {
        let dir = tempdir().expect("tempdir");
        let metadata_path = dir.path().join("metadata.db");
        let store = SnapshotStore::open_local(&metadata_path.display().to_string())
            .await
            .expect("open metadata store");
        let snapshot = store
            .create_or_get_snapshot(SnapshotKey::new(
                "/repo",
                "/repo/.worktrees/compat-doctests",
                "abc123",
                "x86_64-unknown-linux-gnu",
                ["default"],
                "dev",
            ))
            .await
            .expect("create snapshot");
        let chunks = RustChunker::new(120)
            .chunk_workspace(&fixture_root())
            .expect("chunk workspace");

        store
            .replace_chunks(&snapshot.id, &chunks)
            .await
            .expect("persist chunks");
        let loaded = store.load_chunks(&snapshot.id).await.expect("load chunks");

        assert!(loaded.iter().any(|chunk| {
            chunk.chunk_kind == "ExampleFile"
                && chunk
                    .retrieval_markers
                    .iter()
                    .any(|marker| marker == "example")
                && chunk.file_path.ends_with("examples/demo.rs")
        }));
        assert!(loaded.iter().any(|chunk| {
            chunk.chunk_kind == "Doctest"
                && chunk
                    .retrieval_markers
                    .iter()
                    .any(|marker| marker == "doctest")
                && chunk
                    .docs_text
                    .as_deref()
                    .unwrap_or_default()
                    .contains("Adds two numbers")
        }));
    });
}
