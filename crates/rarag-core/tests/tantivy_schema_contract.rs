use std::path::PathBuf;

use rarag_core::chunking::RustChunker;
use rarag_core::indexing::TantivyChunkStore;
use tantivy::Index;
use tempfile::tempdir;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace root")
        .join("tests/fixtures/compat_repo")
}

#[test]
fn maps_chunk_fields_to_rich_lexical_document() {
    let dir = tempdir().expect("tempdir");
    let store = TantivyChunkStore::open(dir.path()).expect("open tantivy");
    let chunks = RustChunker::new(120)
        .chunk_workspace(&fixture_root())
        .expect("chunk workspace");

    store
        .index_chunks("snapshot-rich-schema", &chunks)
        .expect("index chunks");

    let index = Index::open_in_dir(dir.path()).expect("open tantivy index");
    let schema = index.schema();

    assert!(schema.get_field("symbol_name").is_ok());
    assert!(schema.get_field("docs_text").is_ok());
    assert!(schema.get_field("signature_text").is_ok());
    assert!(schema.get_field("retrieval_markers").is_ok());
    assert!(schema.get_field("workflow_hints").is_ok());
}
