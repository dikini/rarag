mod eval;
mod neighborhood;
mod query;
mod rerank;

use crate::config::{DocumentSourceRule, DocumentSourcesConfig, RetrievalConfig};
use crate::config::{ObservabilityConfig, ObservabilityVerbosity};
use crate::embeddings::EmbeddingProvider;
use crate::indexing::{LanceDbPointStore, TantivyChunkStore};
use crate::metadata::{
    CandidateObservationRecord, QueryAuditRecord, QueryObservationRecord, SnapshotStore,
};
use neighborhood::assemble_neighborhood;
pub use query::{QueryMode, RetrievalRequest, RetrievalResponse, RetrievedChunk};
pub use eval::{EvalTaskFixture, load_eval_task_fixtures};
use rerank::{Candidate, RankedCandidate, rerank_candidates};

pub struct RepositoryRetriever<'a, P> {
    metadata: &'a SnapshotStore,
    tantivy: &'a TantivyChunkStore,
    lancedb: &'a LanceDbPointStore,
    provider: &'a P,
    retrieval: RetrievalConfig,
    observability: ObservabilityConfig,
    document_sources: DocumentSourcesConfig,
}

impl<'a, P> RepositoryRetriever<'a, P>
where
    P: EmbeddingProvider,
{
    pub fn new(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        lancedb: &'a LanceDbPointStore,
        provider: &'a P,
    ) -> Self {
        Self::new_with_config(
            metadata,
            tantivy,
            lancedb,
            provider,
            &RetrievalConfig::default(),
        )
    }

    pub fn new_with_settings(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        lancedb: &'a LanceDbPointStore,
        provider: &'a P,
        retrieval: &RetrievalConfig,
        observability: &ObservabilityConfig,
    ) -> Self {
        Self::new_with_full_settings(
            metadata,
            tantivy,
            lancedb,
            provider,
            retrieval,
            observability,
            &DocumentSourcesConfig::default(),
        )
    }

    pub fn new_with_full_settings(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        lancedb: &'a LanceDbPointStore,
        provider: &'a P,
        retrieval: &RetrievalConfig,
        observability: &ObservabilityConfig,
        document_sources: &DocumentSourcesConfig,
    ) -> Self {
        Self {
            metadata,
            tantivy,
            lancedb,
            provider,
            retrieval: retrieval.clone(),
            observability: observability.clone(),
            document_sources: document_sources.clone(),
        }
    }

    pub fn new_with_config(
        metadata: &'a SnapshotStore,
        tantivy: &'a TantivyChunkStore,
        lancedb: &'a LanceDbPointStore,
        provider: &'a P,
        retrieval: &RetrievalConfig,
    ) -> Self {
        Self::new_with_settings(
            metadata,
            tantivy,
            lancedb,
            provider,
            retrieval,
            &ObservabilityConfig::default(),
        )
    }

    pub async fn retrieve(&self, request: RetrievalRequest) -> Result<RetrievalResponse, String> {
        let mut all_chunks = self.metadata.load_chunks(&request.snapshot_id).await?;
        backfill_document_rank_weights(&mut all_chunks, &self.document_sources.rules);
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
        if request.include_history {
            let history = self.history_candidates(&request).await?;
            if history.is_empty() {
                warnings.push("history selector requested but no history candidates were found".to_string());
            }
            candidates.extend(history);
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
            )
            .with_eval(
                request.eval_task_id.clone(),
                evidence_class_coverage(&items),
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
        self.lancedb
            .search_snapshot(
                &request.snapshot_id,
                &query_vector,
                request.effective_limit(),
            )
            .await
    }

    async fn history_candidates(&self, request: &RetrievalRequest) -> Result<Vec<Candidate>, String> {
        let mut nodes = self.metadata.load_history_nodes(&request.snapshot_id).await?;
        if nodes.is_empty() {
            return Ok(Vec::new());
        }
        let edges = self.metadata.load_lineage_edges(&request.snapshot_id).await?;
        let cap = request.history_max_nodes.unwrap_or(8).max(1);
        if nodes.len() > cap {
            nodes = nodes.split_off(nodes.len() - cap);
        }
        let query_terms: Vec<String> = request
            .query_text
            .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
            .filter(|term| !term.is_empty())
            .map(|term| term.to_ascii_lowercase())
            .collect();

        Ok(nodes
            .into_iter()
            .map(|node| {
                let summary_lower = node.summary.to_ascii_lowercase();
                let overlaps = query_terms
                    .iter()
                    .filter(|term| summary_lower.contains(term.as_str()))
                    .count() as f32;
                let mut evidence = vec!["history_node".to_string()];
                if edges
                    .iter()
                    .any(|edge| edge.from_node_id == node.node_id || edge.to_node_id == node.node_id)
                {
                    evidence.push("lineage_edge".to_string());
                }
                Candidate {
                    chunk: crate::metadata::ChunkRecord {
                        chunk_id: format!("history:{}", node.node_id),
                        snapshot_id: request.snapshot_id.clone(),
                        chunk_kind: "HistoryNode".to_string(),
                        symbol_path: node.subject.clone(),
                        symbol_name: node.subject.clone(),
                        owning_symbol_header: None,
                        docs_text: None,
                        signature_text: None,
                        parent_symbol_path: Some("history".to_string()),
                        retrieval_markers: vec!["history".to_string()],
                        repository_state_hints: vec!["history".to_string()],
                        file_path: format!("history/{}", node.node_id),
                        start_byte: 0,
                        end_byte: 0,
                        text: node.summary,
                    },
                    score: 6.0 + overlaps,
                    evidence,
                }
            })
            .collect())
    }
}

fn backfill_document_rank_weights(chunks: &mut [crate::metadata::ChunkRecord], rules: &[DocumentSourceRule]) {
    for chunk in chunks {
        if !chunk
            .retrieval_markers
            .iter()
            .any(|marker| marker == "document")
        {
            continue;
        }
        if chunk
            .retrieval_markers
            .iter()
            .any(|marker| marker.starts_with("doc_rank_weight:"))
        {
            continue;
        }
        let Some(weight) = classify_doc_weight_from_path(&chunk.file_path, rules) else {
            continue;
        };
        chunk.retrieval_markers
            .push(format!("doc_rank_weight:{weight:.3}"));
    }
}

fn classify_doc_weight_from_path(path: &str, rules: &[DocumentSourceRule]) -> Option<f32> {
    let normalized = path.replace('\\', "/");
    for rule in rules {
        if path_matches_glob(&normalized, &rule.path_glob) {
            return Some(rule.weight);
        }
    }
    None
}

fn path_matches_glob(path: &str, glob: &str) -> bool {
    if let Some(prefix) = glob.strip_suffix("/**") {
        return path.starts_with(prefix);
    }
    path == glob
}

fn evidence_class_coverage(items: &[RetrievedChunk]) -> Vec<String> {
    let mut classes = Vec::new();
    for item in items {
        let class = match item.chunk.chunk_kind.as_str() {
            "DocumentBlock" | "TaskRow" => "document",
            "HistoryNode" => "history",
            _ => "code",
        };
        if !classes.iter().any(|existing| existing == class) {
            classes.push(class.to_string());
        }
    }
    classes
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
