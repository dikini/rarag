mod qdrant_store;
mod tantivy_store;

use crate::chunking::Chunk;
use crate::embeddings::EmbeddingProvider;
use crate::metadata::{IndexingRunRecord, SnapshotStore};
use crate::semantic::SemanticEdge;

pub use qdrant_store::QdrantPointStore;
pub use qdrant_store::VectorSearchHit;
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
    qdrant: &'a QdrantPointStore,
    provider: &'a P,
}

impl<'a, P> ChunkIndexer<'a, P>
where
    P: EmbeddingProvider,
{
    pub fn new(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        qdrant: &'a QdrantPointStore,
        provider: &'a P,
    ) -> Self {
        Self {
            metadata,
            tantivy,
            qdrant,
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
        self.metadata
            .record_indexing_run(IndexingRunRecord::new(
                snapshot_id,
                "started",
                u64::try_from(chunks.len()).map_err(|err| err.to_string())?,
            ))
            .await?;
        self.metadata.replace_chunks(snapshot_id, chunks).await?;
        self.metadata.replace_edges(snapshot_id, edges).await?;
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
            .qdrant
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

    pub fn qdrant_store(&self) -> &QdrantPointStore {
        self.qdrant
    }
}
