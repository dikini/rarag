use std::collections::HashMap;

use crate::config::RerankWeightsConfig;
use crate::metadata::ChunkRecord;
use crate::retrieval::query::{QueryMode, RetrievedChunk};
use crate::worktree::WorktreeChanges;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub chunk: ChunkRecord,
    pub score: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RankedCandidate {
    pub item: RetrievedChunk,
    pub base_score: f32,
    pub query_mode_bias: f32,
    pub worktree_diff_bias: f32,
    pub matched_worktree: bool,
}

pub fn rerank_candidates(
    snapshot_id: &str,
    query_mode: QueryMode,
    weights: &RerankWeightsConfig,
    worktree_changes: &WorktreeChanges,
    candidates: Vec<Candidate>,
) -> Vec<RankedCandidate> {
    let mut merged: HashMap<String, Candidate> = HashMap::new();

    for candidate in candidates {
        let entry = merged
            .entry(candidate.chunk.chunk_id.clone())
            .or_insert_with(|| Candidate {
                chunk: candidate.chunk.clone(),
                score: candidate.score,
                evidence: Vec::new(),
            });
        entry.score = entry.score.max(candidate.score);
        for item in candidate.evidence {
            if !entry.evidence.contains(&item) {
                entry.evidence.push(item);
            }
        }
    }

    let mut ranked: Vec<_> = merged
        .into_values()
        .map(|mut candidate| {
            let base_score = candidate.score;
            let query_mode_bias = query_mode_bias(query_mode, &candidate.chunk, weights);
            candidate.score += query_mode_bias;
            let matched_worktree = worktree_changes.matches(&candidate.chunk.file_path);
            let worktree_diff_bias = if matched_worktree {
                worktree_diff_bias(query_mode, weights)
            } else {
                0.0
            };
            if matched_worktree {
                candidate.score += worktree_diff_bias;
                if !candidate
                    .evidence
                    .iter()
                    .any(|item| item == "worktree_diff")
                {
                    candidate.evidence.push("worktree_diff".to_string());
                }
            }
            RankedCandidate {
                item: RetrievedChunk {
                    snapshot_id: snapshot_id.to_string(),
                    chunk: candidate.chunk,
                    score: candidate.score,
                    evidence: candidate.evidence,
                },
                base_score,
                query_mode_bias,
                worktree_diff_bias,
                matched_worktree,
            }
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .item
            .score
            .total_cmp(&left.item.score)
            .then_with(|| left.item.chunk.file_path.cmp(&right.item.chunk.file_path))
            .then_with(|| left.item.chunk.start_byte.cmp(&right.item.chunk.start_byte))
    });
    ranked
}

fn query_mode_bias(
    query_mode: QueryMode,
    chunk: &ChunkRecord,
    weights: &RerankWeightsConfig,
) -> f32 {
    match query_mode {
        QueryMode::UnderstandSymbol => {
            let symbol_bias = if chunk.chunk_kind == "Symbol" {
                weights.understand_symbol_symbol
            } else {
                0.0
            };
            symbol_bias + document_normativity_bias(chunk)
        }
        QueryMode::ImplementAdjacent => {
            if chunk.chunk_kind == "BodyRegion" {
                weights.implement_adjacent_body_region
            } else {
                0.0
            }
        }
        QueryMode::BoundedRefactor => {
            if is_test_like(chunk) {
                weights.bounded_refactor_test_like
            } else {
                weights.bounded_refactor_other
            }
        }
        QueryMode::BlastRadius => {
            if chunk.chunk_kind == "HistoryNode" {
                0.5
            } else if is_test_like(chunk) {
                weights.blast_radius_test_like
            } else {
                weights.blast_radius_other
            }
        }
        QueryMode::FindExamples => {
            if is_example_like(chunk) {
                weights.find_examples_example_like
            } else {
                weights.find_examples_other
            }
        }
    }
}

fn is_test_like(chunk: &ChunkRecord) -> bool {
    chunk.chunk_kind == "TestFunction"
        || chunk
            .retrieval_markers
            .iter()
            .any(|marker| marker == "test" || marker == "doctest")
}

fn is_example_like(chunk: &ChunkRecord) -> bool {
    matches!(
        chunk.chunk_kind.as_str(),
        "TestFunction" | "ExampleFile" | "Doctest"
    ) || chunk
        .retrieval_markers
        .iter()
        .any(|marker| matches!(marker.as_str(), "test" | "example" | "doctest"))
}

fn worktree_diff_bias(query_mode: QueryMode, weights: &RerankWeightsConfig) -> f32 {
    match query_mode {
        QueryMode::BoundedRefactor => weights.worktree_diff_bounded_refactor,
        QueryMode::BlastRadius => weights.worktree_diff_blast_radius,
        QueryMode::ImplementAdjacent => weights.worktree_diff_implement_adjacent,
        QueryMode::FindExamples => weights.worktree_diff_find_examples,
        QueryMode::UnderstandSymbol => weights.worktree_diff_understand_symbol,
    }
}

fn document_normativity_bias(chunk: &ChunkRecord) -> f32 {
    if !chunk
        .retrieval_markers
        .iter()
        .any(|marker| marker == "document")
    {
        return 0.0;
    }
    if let Some(weight) = parse_document_rank_weight(chunk) {
        return weight;
    }
    legacy_document_normativity_bias(chunk)
}

fn parse_document_rank_weight(chunk: &ChunkRecord) -> Option<f32> {
    chunk
        .retrieval_markers
        .iter()
        .find_map(|marker| marker.strip_prefix("doc_rank_weight:"))
        .and_then(|raw| raw.parse::<f32>().ok())
}

fn legacy_document_normativity_bias(chunk: &ChunkRecord) -> f32 {
    if chunk
        .retrieval_markers
        .iter()
        .any(|marker| marker == "spec")
    {
        return 1.2;
    }
    if chunk
        .retrieval_markers
        .iter()
        .any(|marker| marker == "ops" || marker == "integrations")
    {
        return 0.8;
    }
    if chunk
        .retrieval_markers
        .iter()
        .any(|marker| marker == "plan")
    {
        return -0.2;
    }
    if chunk
        .retrieval_markers
        .iter()
        .any(|marker| marker == "changelog")
    {
        return -0.4;
    }
    0.2
}

#[cfg(test)]
mod tests {
    use super::{Candidate, rerank_candidates};
    use crate::config::RerankWeightsConfig;
    use crate::metadata::ChunkRecord;
    use crate::retrieval::QueryMode;
    use crate::worktree::WorktreeChanges;

    fn chunk(chunk_id: &str, chunk_kind: &str, file_path: &str, markers: &[&str]) -> ChunkRecord {
        ChunkRecord {
            chunk_id: chunk_id.to_string(),
            snapshot_id: "snapshot-1".to_string(),
            chunk_kind: chunk_kind.to_string(),
            symbol_path: Some(format!("mini_repo::{chunk_id}")),
            symbol_name: Some(chunk_id.to_string()),
            owning_symbol_header: None,
            docs_text: Some(String::new()),
            signature_text: Some(String::new()),
            parent_symbol_path: None,
            retrieval_markers: markers.iter().map(|item| item.to_string()).collect(),
            repository_state_hints: Vec::new(),
            file_path: file_path.to_string(),
            start_byte: 0,
            end_byte: 10,
            text: chunk_id.to_string(),
        }
    }

    #[test]
    fn default_weights_preserve_symbol_priority() {
        let items = rerank_candidates(
            "snapshot-1",
            QueryMode::UnderstandSymbol,
            &RerankWeightsConfig::default(),
            &WorktreeChanges::default(),
            vec![
                Candidate {
                    chunk: chunk("body", "BodyRegion", "src/lib.rs", &[]),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
                Candidate {
                    chunk: chunk("symbol", "Symbol", "src/lib.rs", &[]),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
            ],
        );

        assert_eq!(items[0].item.chunk.chunk_kind, "Symbol");
        assert!(items[0].item.score > items[1].item.score);
    }

    #[test]
    fn override_weights_change_rank_order() {
        let weights = RerankWeightsConfig {
            understand_symbol_symbol: -2.0,
            ..RerankWeightsConfig::default()
        };
        let items = rerank_candidates(
            "snapshot-1",
            QueryMode::UnderstandSymbol,
            &weights,
            &WorktreeChanges::default(),
            vec![
                Candidate {
                    chunk: chunk("body", "BodyRegion", "src/lib.rs", &[]),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
                Candidate {
                    chunk: chunk("symbol", "Symbol", "src/lib.rs", &[]),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
            ],
        );

        assert_eq!(items[0].item.chunk.chunk_kind, "BodyRegion");
        assert!(items[0].item.score > items[1].item.score);
    }

    #[test]
    fn document_rank_weight_marker_changes_order() {
        let items = rerank_candidates(
            "snapshot-1",
            QueryMode::UnderstandSymbol,
            &RerankWeightsConfig::default(),
            &WorktreeChanges::default(),
            vec![
                Candidate {
                    chunk: chunk(
                        "plan-doc",
                        "DocumentBlock",
                        "docs/plans/future.md",
                        &["document", "plan", "doc_rank_weight:2.0"],
                    ),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
                Candidate {
                    chunk: chunk(
                        "spec-doc",
                        "DocumentBlock",
                        "docs/specs/current.md",
                        &["document", "spec", "doc_rank_weight:0.1"],
                    ),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
            ],
        );

        assert_eq!(items[0].item.chunk.chunk_id, "plan-doc");
        assert!(items[0].item.score > items[1].item.score);
    }

    #[test]
    fn legacy_document_markers_without_rank_weight_still_apply_bias() {
        let items = rerank_candidates(
            "snapshot-1",
            QueryMode::UnderstandSymbol,
            &RerankWeightsConfig::default(),
            &WorktreeChanges::default(),
            vec![
                Candidate {
                    chunk: chunk(
                        "plan-doc",
                        "DocumentBlock",
                        "docs/plans/future.md",
                        &["document", "plan"],
                    ),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
                Candidate {
                    chunk: chunk(
                        "spec-doc",
                        "DocumentBlock",
                        "docs/specs/current.md",
                        &["document", "spec"],
                    ),
                    score: 1.0,
                    evidence: vec!["lexical_bm25".to_string()],
                },
            ],
        );

        assert_eq!(items[0].item.chunk.chunk_id, "spec-doc");
        assert!(items[0].item.score > items[1].item.score);
    }
}
