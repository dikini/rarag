mod neighborhood;
mod query;
mod rerank;

use crate::config::RetrievalConfig;
use crate::config::{ObservabilityConfig, ObservabilityVerbosity};
use crate::embeddings::EmbeddingProvider;
use crate::indexing::{QdrantPointStore, TantivyChunkStore};
use crate::metadata::{
    CandidateObservationRecord, QueryAuditRecord, QueryObservationRecord, SnapshotStore,
};
use neighborhood::assemble_neighborhood;
pub use query::{QueryMode, RetrievalRequest, RetrievalResponse, RetrievedChunk};
use rerank::{Candidate, RankedCandidate, rerank_candidates};

pub struct RepositoryRetriever<'a, P> {
    metadata: &'a SnapshotStore,
    tantivy: &'a TantivyChunkStore,
    qdrant: &'a QdrantPointStore,
    provider: &'a P,
    retrieval: RetrievalConfig,
    observability: ObservabilityConfig,
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
        Self::new_with_config(
            metadata,
            tantivy,
            qdrant,
            provider,
            &RetrievalConfig::default(),
        )
    }

    pub fn new_with_settings(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        qdrant: &'a QdrantPointStore,
        provider: &'a P,
        retrieval: &RetrievalConfig,
        observability: &ObservabilityConfig,
    ) -> Self {
        Self {
            metadata,
            tantivy,
            qdrant,
            provider,
            retrieval: retrieval.clone(),
            observability: observability.clone(),
        }
    }

    pub fn new_with_config(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        qdrant: &'a QdrantPointStore,
        provider: &'a P,
        retrieval: &RetrievalConfig,
    ) -> Self {
        Self::new_with_settings(
            metadata,
            tantivy,
            qdrant,
            provider,
            retrieval,
            &ObservabilityConfig::default(),
        )
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

        let mut candidates = assemble_neighborhood(
            &request,
            &self.retrieval.neighborhood,
            &all_chunks,
            &seed_chunks,
            &all_edges,
        );
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

        let ranked = rerank_candidates(
            &request.snapshot_id,
            request.query_mode,
            &self.retrieval.rerank,
            &request.worktree_changes,
            candidates,
        );
        let items: Vec<_> = ranked
            .iter()
            .take(request.effective_limit())
            .map(|candidate| candidate.item.clone())
            .collect();

        self.metadata
            .record_query_audit(QueryAuditRecord::new(
                &request.snapshot_id,
                request.query_mode.as_str(),
                &request.query_text,
                u64::try_from(items.len()).map_err(|err| err.to_string())?,
            ))
            .await?;

        if self.observability.enabled {
            let observation_id = observation_id(&request.snapshot_id);
            let observation = QueryObservationRecord::new(
                observation_id.clone(),
                request.snapshot_id.clone(),
                request.query_mode.as_str(),
                request.query_text.clone(),
                request.symbol_path.clone(),
                request.worktree_changes.paths().to_vec(),
                warnings.clone(),
                u64::try_from(items.len()).map_err(|err| err.to_string())?,
                self.retrieval.clone(),
                self.observability.clone(),
            );
            let candidate_records =
                observation_candidates(&observation_id, &ranked, request.effective_limit())?;
            emit_observation_logs(&observation, &candidate_records);
            if let Err(err) = self
                .metadata
                .record_query_observation(observation, &candidate_records)
                .await
            {
                warnings.push(format!("query observation capture failed: {err}"));
            }
        }

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

fn observation_id(snapshot_id: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("obs-{snapshot_id}-{nanos}")
}

fn observation_candidates(
    observation_id: &str,
    ranked: &[RankedCandidate],
    returned_limit: usize,
) -> Result<Vec<CandidateObservationRecord>, String> {
    ranked
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            Ok(CandidateObservationRecord::new(
                observation_id.to_string(),
                candidate.item.chunk.chunk_id.clone(),
                candidate.item.chunk.chunk_kind.clone(),
                candidate.item.chunk.symbol_path.clone(),
                candidate.item.chunk.file_path.clone(),
                candidate.item.evidence.clone(),
                candidate.item.chunk.retrieval_markers.clone(),
                u32::try_from(index + 1).map_err(|err| err.to_string())?,
                index < returned_limit,
                candidate.matched_worktree,
                candidate.base_score,
                candidate.query_mode_bias,
                candidate.worktree_diff_bias,
                candidate.item.score,
            ))
        })
        .collect()
}

fn emit_observation_logs(
    observation: &QueryObservationRecord,
    candidates: &[CandidateObservationRecord],
) {
    if observation.observability.verbosity == ObservabilityVerbosity::Off {
        return;
    }

    let summary = serde_json::json!({
        "event": "retrieval_observation",
        "observation_id": observation.observation_id,
        "snapshot_id": observation.snapshot_id,
        "query_mode": observation.query_mode,
        "query_text": observation.query_text,
        "symbol_path": observation.symbol_path,
        "changed_paths": observation.changed_paths,
        "warnings": observation.warnings,
        "result_count": observation.result_count,
        "candidate_count": candidates.len(),
    });
    eprintln!(
        "{}",
        serde_json::to_string(&summary).expect("serialize retrieval observation summary")
    );

    if observation.observability.verbosity == ObservabilityVerbosity::Detailed {
        for candidate in candidates {
            let event = serde_json::json!({
                "event": "retrieval_candidate",
                "observation_id": observation.observation_id,
                "rank": candidate.rank,
                "returned": candidate.returned,
                "chunk_id": candidate.chunk_id,
                "chunk_kind": candidate.chunk_kind,
                "symbol_path": candidate.symbol_path,
                "file_path": candidate.file_path,
                "evidence": candidate.evidence,
                "retrieval_markers": candidate.retrieval_markers,
                "matched_worktree": candidate.matched_worktree,
                "base_score": candidate.base_score,
                "query_mode_bias": candidate.query_mode_bias,
                "worktree_diff_bias": candidate.worktree_diff_bias,
                "final_score": candidate.final_score,
            });
            eprintln!(
                "{}",
                serde_json::to_string(&event).expect("serialize retrieval candidate event")
            );
        }
    }
}
