mod neighborhood;
mod query;
mod rerank;

use crate::embeddings::EmbeddingProvider;
use crate::indexing::{QdrantPointStore, TantivyChunkStore};
use crate::metadata::{QueryAuditRecord, SnapshotStore};
use neighborhood::assemble_neighborhood;
pub use query::{QueryMode, RetrievalRequest, RetrievalResponse, RetrievedChunk, WorkflowPhase};
use rerank::{Candidate, rerank_candidates};

pub struct RepositoryRetriever<'a, P> {
    metadata: &'a SnapshotStore,
    tantivy: &'a TantivyChunkStore,
    qdrant: &'a QdrantPointStore,
    provider: &'a P,
}

impl<'a, P> RepositoryRetriever<'a, P>
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

    pub async fn retrieve(&self, request: RetrievalRequest) -> Result<RetrievalResponse, String> {
        let all_chunks = self.metadata.load_chunks(&request.snapshot_id).await?;
        let all_edges = self.metadata.load_edges(&request.snapshot_id).await?;
        let mut warnings = Vec::new();

        let seed_chunks = if let Some(symbol_path) = request.symbol_path.as_deref() {
            let hits = self.tantivy.search_exact_symbol_for_snapshot(
                &request.snapshot_id,
                symbol_path,
                request.effective_limit(),
            )?;
            let chunk_ids: Vec<_> = hits.into_iter().map(|hit| hit.chunk_id).collect();
            let seeds: Vec<_> = all_chunks
                .iter()
                .filter(|chunk| chunk_ids.iter().any(|id| id == &chunk.chunk_id))
                .cloned()
                .collect();
            if seeds.is_empty() {
                warnings.push("exact symbol match not found".to_string());
            }
            seeds
        } else {
            warnings.push("symbol path not provided; exact symbol retrieval skipped".to_string());
            Vec::new()
        };

        let lexical_hits = self.tantivy.search_text_for_snapshot(
            &request.snapshot_id,
            &request.query_text,
            request.effective_limit(),
        )?;
        if lexical_hits.is_empty() {
            warnings.push("lexical bm25 search returned no snapshot-local candidates".to_string());
        }

        let semantic_hits = match self.semantic_candidates(&request).await {
            Ok(hits) => {
                if hits.is_empty() {
                    warnings.push(
                        "semantic vector search returned no snapshot-local candidates".to_string(),
                    );
                }
                hits
            }
            Err(err) => {
                warnings.push(format!("semantic vector search unavailable: {err}"));
                Vec::new()
            }
        };

        let mut candidates = assemble_neighborhood(&request, &all_chunks, &seed_chunks, &all_edges);
        for hit in lexical_hits {
            if let Some(chunk) = all_chunks
                .iter()
                .find(|chunk| chunk.chunk_id == hit.chunk_id)
            {
                candidates.push(Candidate {
                    chunk: chunk.clone(),
                    score: hit.score + 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                });
            }
        }
        for hit in semantic_hits {
            if let Some(chunk) = all_chunks
                .iter()
                .find(|chunk| chunk.chunk_id == hit.chunk_id)
            {
                candidates.push(Candidate {
                    chunk: chunk.clone(),
                    score: hit.score + 1.5,
                    evidence: vec!["semantic_vector".to_string()],
                });
            }
        }

        let items = rerank_candidates(
            &request.snapshot_id,
            request.query_mode,
            request.workflow_phase,
            &request.worktree_changes,
            candidates,
            request.effective_limit(),
        );

        self.metadata
            .record_query_audit(QueryAuditRecord::new(
                &request.snapshot_id,
                request.query_mode.as_str(),
                &request.query_text,
                u64::try_from(items.len()).map_err(|err| err.to_string())?,
            ))
            .await?;

        Ok(RetrievalResponse { items, warnings })
    }

    async fn semantic_candidates(
        &self,
        request: &RetrievalRequest,
    ) -> Result<Vec<crate::indexing::VectorSearchHit>, String> {
        let vectors = self
            .provider
            .embed_texts(std::slice::from_ref(&request.query_text))?;
        let query_vector = vectors
            .into_iter()
            .next()
            .ok_or_else(|| "embedding provider returned no query vector".to_string())?;
        self.qdrant
            .search_snapshot(
                &request.snapshot_id,
                &query_vector,
                request.effective_limit(),
            )
            .await
    }
}
