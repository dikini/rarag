mod lancedb_store;
mod tantivy_store;

use crate::chunking::Chunk;
use crate::embeddings::EmbeddingProvider;
use crate::metadata::{
    DocumentBlockRecord, HistoryNodeRecord, IndexingRunRecord, LineageEdgeRecord, SnapshotStore,
};
use crate::semantic::SemanticEdge;

pub use lancedb_store::LanceDbPointStore;
pub use lancedb_store::VectorSearchHit;
pub use tantivy_store::{IndexedDocument, TantivyChunkStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReindexCounts {
    pub metadata_rows: usize,
    pub lexical_docs: usize,
    pub vector_points: usize,
}

pub struct ChunkIndexer<'a, P> {
    metadata: &'a SnapshotStore,
    tantivy: &'a TantivyChunkStore,
    lancedb: &'a LanceDbPointStore,
    provider: &'a P,
}

impl<'a, P> ChunkIndexer<'a, P>
where
    P: EmbeddingProvider,
{
    pub fn new(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        lancedb: &'a LanceDbPointStore,
        provider: &'a P,
    ) -> Self {
        Self {
            metadata,
            tantivy,
            lancedb,
            provider,
        }
    }

    pub async fn reindex_snapshot(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
    ) -> Result<ReindexCounts, String> {
        self.reindex_snapshot_with_semantics(snapshot_id, chunks, &[])
            .await
    }

    pub async fn reindex_snapshot_with_semantics(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
        edges: &[SemanticEdge],
    ) -> Result<ReindexCounts, String> {
        self.reindex_snapshot_with_history(snapshot_id, chunks, edges, &[], &[])
            .await
    }

    pub async fn reindex_snapshot_with_history(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
        edges: &[SemanticEdge],
        history_nodes: &[HistoryNodeRecord],
        lineage_edges: &[LineageEdgeRecord],
    ) -> Result<ReindexCounts, String> {
        self.metadata
            .record_indexing_run(IndexingRunRecord::new(
                snapshot_id,
                "started",
                u64::try_from(chunks.len()).map_err(|err| err.to_string())?,
            ))
            .await?;
        self.metadata.replace_chunks(snapshot_id, chunks).await?;
        let document_blocks = document_blocks_from_chunks(snapshot_id, chunks);
        self.metadata
            .replace_document_blocks(snapshot_id, &document_blocks)
            .await?;
        self.metadata.replace_edges(snapshot_id, edges).await?;
        self.metadata
            .replace_history_nodes(snapshot_id, history_nodes)
            .await?;
        self.metadata
            .replace_lineage_edges(snapshot_id, lineage_edges)
            .await?;
        let result = self.reindex_snapshot_inner(snapshot_id, chunks).await;
        let status = if result.is_ok() {
            "completed"
        } else {
            "failed"
        };
        self.metadata
            .record_indexing_run(IndexingRunRecord::new(
                snapshot_id,
                status,
                u64::try_from(chunks.len()).map_err(|err| err.to_string())?,
            ))
            .await?;
        result
    }

    async fn reindex_snapshot_inner(
        &self,
        snapshot_id: &str,
        chunks: &[Chunk],
    ) -> Result<ReindexCounts, String> {
        let lexical_docs = self.tantivy.index_chunks(snapshot_id, chunks)?;
        let texts: Vec<String> = chunks.iter().map(|chunk| chunk.text.clone()).collect();
        let vectors = self.provider.embed_texts(&texts)?;
        let vector_points = self
            .lancedb
            .replace_snapshot(snapshot_id, chunks, vectors)
            .await?;
        let metadata_rows = self.metadata.chunk_count(snapshot_id).await?;

        Ok(ReindexCounts {
            metadata_rows,
            lexical_docs,
            vector_points,
        })
    }

    pub fn tantivy_store(&self) -> &TantivyChunkStore {
        self.tantivy
    }

    pub fn lancedb_store(&self) -> &LanceDbPointStore {
        self.lancedb
    }
}

fn document_blocks_from_chunks(snapshot_id: &str, chunks: &[Chunk]) -> Vec<DocumentBlockRecord> {
    chunks
        .iter()
        .filter_map(|chunk| match chunk.kind {
            crate::chunking::ChunkKind::DocumentBlock | crate::chunking::ChunkKind::TaskRow => {
                let document_kind = chunk
                    .retrieval_markers
                    .iter()
                    .find(|marker| marker.as_str() != "document")
                    .cloned()
                    .unwrap_or_else(|| "documentation".to_string());
                let parser = if matches!(chunk.kind, crate::chunking::ChunkKind::TaskRow) {
                    "csv"
                } else {
                    "markdown"
                };
                Some(DocumentBlockRecord::new(
                    chunk.id.clone(),
                    snapshot_id.to_string(),
                    chunk.file_path.display().to_string(),
                    document_kind,
                    parser,
                    chunk
                        .symbol_path
                        .as_deref()
                        .map(|path| path.split("::").map(ToString::to_string).collect())
                        .unwrap_or_else(Vec::new),
                    chunk.span.start_byte,
                    chunk.span.end_byte,
                    chunk.text.clone(),
                ))
            }
            _ => None,
        })
        .collect()
}
