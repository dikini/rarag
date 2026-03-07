use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::doc;
use tantivy::query::{QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Index, IndexReader, ReloadPolicy, TantivyDocument, Term};

use crate::chunking::{Chunk, ChunkKind};

#[derive(Debug, Clone, PartialEq)]
pub struct IndexedDocument {
    pub chunk_id: String,
    pub symbol_path: Option<String>,
    pub score: f32,
}

pub struct TantivyChunkStore {
    index: Index,
    reader: IndexReader,
    chunk_id: tantivy::schema::Field,
    snapshot_id: tantivy::schema::Field,
    symbol_path: tantivy::schema::Field,
    file_path: tantivy::schema::Field,
    kind: tantivy::schema::Field,
    text: tantivy::schema::Field,
}

impl TantivyChunkStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(path).map_err(|err| err.to_string())?;

        let mut schema_builder = Schema::builder();
        let chunk_id = schema_builder.add_text_field("chunk_id", STRING | STORED);
        let snapshot_id = schema_builder.add_text_field("snapshot_id", STRING | STORED);
        let symbol_path = schema_builder.add_text_field("symbol_path", STRING | STORED);
        let file_path = schema_builder.add_text_field("file_path", STRING | STORED);
        let kind = schema_builder.add_text_field("kind", STRING | STORED);
        let text = schema_builder.add_text_field("text", TEXT | STORED);
        let schema = schema_builder.build();

        let directory = MmapDirectory::open(path).map_err(|err| err.to_string())?;
        let index = Index::open_or_create(directory, schema).map_err(|err| err.to_string())?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|err| err.to_string())?;

        Ok(Self {
            index,
            reader,
            chunk_id,
            snapshot_id,
            symbol_path,
            file_path,
            kind,
            text,
        })
    }

    pub fn index_chunks(&self, snapshot_id: &str, chunks: &[Chunk]) -> Result<usize, String> {
        let mut writer = self
            .index
            .writer(15_000_000)
            .map_err(|err| err.to_string())?;
        writer.delete_term(Term::from_field_text(self.snapshot_id, snapshot_id));

        for chunk in chunks {
            writer
                .add_document(doc!(
                    self.chunk_id => chunk.id.clone(),
                    self.snapshot_id => snapshot_id.to_string(),
                    self.symbol_path => chunk.symbol_path.clone().unwrap_or_default(),
                    self.file_path => chunk.file_path.display().to_string(),
                    self.kind => chunk_kind_name(&chunk.kind).to_string(),
                    self.text => chunk.text.clone(),
                ))
                .map_err(|err| err.to_string())?;
        }

        writer.commit().map_err(|err| err.to_string())?;
        self.reader.reload().map_err(|err| err.to_string())?;
        Ok(chunks.len())
    }

    pub fn document_count(&self) -> Result<usize, String> {
        Ok(self.reader.searcher().num_docs() as usize)
    }

    pub fn search_exact_symbol(
        &self,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<IndexedDocument>, String> {
        self.search_exact_symbol_in_snapshot(None, symbol, limit)
    }

    pub fn search_exact_symbol_for_snapshot(
        &self,
        snapshot_id: &str,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<IndexedDocument>, String> {
        self.search_exact_symbol_in_snapshot(Some(snapshot_id), symbol, limit)
    }

    pub fn search_text_for_snapshot(
        &self,
        snapshot_id: &str,
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<IndexedDocument>, String> {
        let parser = QueryParser::for_index(
            &self.index,
            vec![self.text, self.symbol_path, self.file_path, self.kind],
        );
        let query = parser
            .parse_query(query_text)
            .map_err(|err| err.to_string())?;
        let searcher = self.reader.searcher();
        let docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|err| err.to_string())?;

        docs.into_iter()
            .map(|(score, address)| {
                let document: TantivyDocument =
                    searcher.doc(address).map_err(|err| err.to_string())?;
                let document_snapshot_id = document
                    .get_first(self.snapshot_id)
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if document_snapshot_id != snapshot_id {
                    return Ok(None);
                }

                Ok(Some(IndexedDocument {
                    chunk_id: document
                        .get_first(self.chunk_id)
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    symbol_path: document
                        .get_first(self.symbol_path)
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string),
                    score,
                }))
            })
            .filter_map(Result::transpose)
            .collect()
    }

    fn search_exact_symbol_in_snapshot(
        &self,
        snapshot_id: Option<&str>,
        symbol: &str,
        limit: usize,
    ) -> Result<Vec<IndexedDocument>, String> {
        let term = Term::from_field_text(self.symbol_path, symbol);
        let query = TermQuery::new(term, IndexRecordOption::Basic);
        let searcher = self.reader.searcher();
        let docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|err| err.to_string())?;

        docs.into_iter()
            .map(|(_, address)| {
                let document: TantivyDocument =
                    searcher.doc(address).map_err(|err| err.to_string())?;
                let document_snapshot_id = document
                    .get_first(self.snapshot_id)
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if snapshot_id.is_some_and(|expected| expected != document_snapshot_id) {
                    return Ok(None);
                }

                Ok(Some(IndexedDocument {
                    chunk_id: document
                        .get_first(self.chunk_id)
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    symbol_path: document
                        .get_first(self.symbol_path)
                        .and_then(|value| value.as_str())
                        .map(ToString::to_string),
                    score: 1.0,
                }))
            })
            .filter_map(Result::transpose)
            .collect()
    }
}

fn chunk_kind_name(kind: &ChunkKind) -> &'static str {
    match kind {
        ChunkKind::CrateSummary => "crate_summary",
        ChunkKind::ModuleSummary => "module_summary",
        ChunkKind::Symbol => "symbol",
        ChunkKind::BodyRegion => "body_region",
        ChunkKind::TestFunction => "test_function",
    }
}
