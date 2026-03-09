use std::path::PathBuf;

use rarag_core::chunking::{ChunkKind, RustChunker};

fn compat_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/compat_repo")
}

#[test]
fn classifies_spec_plan_ops_changelog_and_tasks_registry_sources() {
    let chunks = RustChunker::new(120)
        .chunk_workspace(&compat_fixture_root())
        .expect("chunk workspace");

    let mut markers = chunks
        .iter()
        .filter(|chunk| matches!(chunk.kind, ChunkKind::DocumentBlock | ChunkKind::TaskRow))
        .flat_map(|chunk| chunk.retrieval_markers.clone())
        .collect::<Vec<_>>();
    markers.sort();
    markers.dedup();

    assert!(markers.iter().any(|entry| entry == "spec"));
    assert!(markers.iter().any(|entry| entry == "plan"));
    assert!(markers.iter().any(|entry| entry == "ops"));
    assert!(markers.iter().any(|entry| entry == "changelog"));
    assert!(markers.iter().any(|entry| entry == "tasks-registry"));
    assert!(chunks.iter().any(|chunk| {
        chunk
            .retrieval_markers
            .iter()
            .any(|marker| marker.starts_with("doc_rank_weight:"))
    }));
}

#[test]
fn extracts_heading_path_and_line_spans() {
    let chunks = RustChunker::new(120)
        .chunk_workspace(&compat_fixture_root())
        .expect("chunk workspace");

    let section = chunks
        .iter()
        .find(|chunk| {
            chunk
                .file_path
                .to_string_lossy()
                .ends_with("docs/specs/current-behavior.md")
                && chunk.text.contains("snapshot-scoped retrieval")
        })
        .expect("spec section chunk");

    assert_eq!(section.kind, ChunkKind::DocumentBlock);
    assert!(
        section
            .symbol_path
            .as_deref()
            .is_some_and(|symbol| symbol.contains("docs::"))
    );
    assert!(section.span.start_byte > 0);
    assert!(section.span.end_byte >= section.span.start_byte);
}

#[test]
fn extracts_tasks_csv_rows_as_structured_blocks() {
    let chunks = RustChunker::new(120)
        .chunk_workspace(&compat_fixture_root())
        .expect("chunk workspace");

    let task_rows = chunks
        .iter()
        .filter(|chunk| {
            matches!(chunk.kind, ChunkKind::TaskRow)
                && chunk
                    .file_path
                    .to_string_lossy()
                    .ends_with("docs/tasks/tasks.csv")
        })
        .collect::<Vec<_>>();

    assert_eq!(task_rows.len(), 2);
    assert!(
        task_rows
            .iter()
            .any(|row| row.text.contains("id: TASK-1") && row.text.contains("status: done"))
    );
}

#[test]
fn sibling_sections_never_merge_into_one_chunk() {
    let chunks = RustChunker::new(120)
        .chunk_workspace(&compat_fixture_root())
        .expect("chunk workspace");

    let spec_sections = chunks
        .iter()
        .filter(|chunk| {
            chunk
                .file_path
                .to_string_lossy()
                .ends_with("docs/specs/current-behavior.md")
                && matches!(chunk.kind, ChunkKind::DocumentBlock)
        })
        .collect::<Vec<_>>();

    assert!(
        spec_sections
            .iter()
            .any(|chunk| chunk.text.contains("Query Contract"))
    );
    assert!(
        spec_sections
            .iter()
            .any(|chunk| chunk.text.contains("## Reload"))
    );
    assert!(
        spec_sections
            .iter()
            .all(|chunk| !(chunk.text.contains("Query Contract")
                && chunk.text.contains("## Reload")))
    );
}
