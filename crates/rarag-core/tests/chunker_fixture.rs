use std::fs;
use std::path::{Path, PathBuf};

use rarag_core::chunking::{ChunkKind, RustChunker};

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

fn source_slice(path: &Path, start: u32, end: u32) -> String {
    let source = fs::read_to_string(path).expect("read source file");
    source[start as usize..end as usize].to_string()
}

#[test]
fn extracts_symbol_chunks_from_fixture() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    let symbol_paths: Vec<_> = chunks
        .iter()
        .filter_map(|chunk| chunk.symbol_path.as_deref())
        .collect();

    assert!(symbol_paths.contains(&"mini_repo::example_sum"));
    assert!(symbol_paths.contains(&"mini_repo::oversized_example"));
    assert!(symbol_paths.contains(&"mini_repo::nested::helper"));
}

#[test]
fn preserves_symbol_header_on_body_split() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    let body_region = chunks
        .iter()
        .find(|chunk| {
            chunk.kind == ChunkKind::BodyRegion
                && chunk.symbol_path.as_deref() == Some("mini_repo::oversized_example")
        })
        .expect("body region chunk");

    assert!(
        body_region
            .owning_symbol_header
            .as_deref()
            .unwrap_or_default()
            .contains("fn oversized_example")
    );
}

#[test]
fn span_text_matches_source_slice() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    for chunk in chunks {
        assert_eq!(
            chunk.text,
            source_slice(&chunk.file_path, chunk.span.start_byte, chunk.span.end_byte)
        );
    }
}

#[test]
fn indexes_fixture_workspace_structurally() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    assert!(chunks.iter().any(|chunk| {
        chunk.kind == ChunkKind::ModuleSummary
            && chunk.symbol_path.as_deref() == Some("mini_repo::nested")
    }));
    assert!(
        chunks
            .iter()
            .any(|chunk| chunk.kind == ChunkKind::TestFunction
                && chunk.symbol_path.as_deref() == Some("mini_repo::tests::example_sum_smoke"))
    );
}

#[test]
fn captures_symbol_docs_and_signature_text() {
    let chunks = RustChunker::new(120)
        .chunk_workspace(&compat_fixture_root())
        .expect("chunk workspace");

    let symbol = chunks
        .iter()
        .find(|chunk| chunk.symbol_path.as_deref() == Some("compat_repo::doc_example_sum"))
        .expect("documented symbol chunk");

    assert_eq!(symbol.symbol_name.as_deref(), Some("doc_example_sum"));
    assert!(
        symbol
            .docs_text
            .as_deref()
            .unwrap_or_default()
            .contains("Adds two numbers together.")
    );
    assert!(
        symbol
            .signature_text
            .as_deref()
            .unwrap_or_default()
            .contains("pub fn doc_example_sum")
    );
}

#[test]
fn body_region_preserves_parent_relationships() {
    let chunks = RustChunker::new(80)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    let body_region = chunks
        .iter()
        .find(|chunk| {
            chunk.kind == ChunkKind::BodyRegion
                && chunk.symbol_path.as_deref() == Some("mini_repo::oversized_example")
        })
        .expect("body region chunk");

    assert_eq!(
        body_region.parent_symbol_path.as_deref(),
        Some("mini_repo::oversized_example")
    );
}
