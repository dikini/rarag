use std::collections::HashMap;

use crate::metadata::ChunkRecord;
use crate::retrieval::query::{QueryMode, RetrievedChunk, WorkflowPhase};
use crate::worktree::WorktreeChanges;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub chunk: ChunkRecord,
    pub score: f32,
    pub evidence: Vec<String>,
}

pub fn rerank_candidates(
    snapshot_id: &str,
    query_mode: QueryMode,
    workflow_phase: WorkflowPhase,
    worktree_changes: &WorktreeChanges,
    candidates: Vec<Candidate>,
    limit: usize,
) -> Vec<RetrievedChunk> {
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
            candidate.score += workflow_phase_bias(workflow_phase, &candidate.chunk);
            candidate.score += query_mode_bias(query_mode, &candidate.chunk);
            if worktree_changes.matches(&candidate.chunk.file_path) {
                candidate.score += worktree_diff_bias(query_mode);
                if !candidate
                    .evidence
                    .iter()
                    .any(|item| item == "worktree_diff")
                {
                    candidate.evidence.push("worktree_diff".to_string());
                }
            }
            RetrievedChunk {
                snapshot_id: snapshot_id.to_string(),
                chunk: candidate.chunk,
                score: candidate.score,
                evidence: candidate.evidence,
            }
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.chunk.file_path.cmp(&right.chunk.file_path))
            .then_with(|| left.chunk.start_byte.cmp(&right.chunk.start_byte))
    });
    ranked.truncate(limit);
    ranked
}

fn workflow_phase_bias(workflow_phase: WorkflowPhase, chunk: &ChunkRecord) -> f32 {
    match workflow_phase {
        WorkflowPhase::WriteTests | WorkflowPhase::Verify => {
            if is_test_like(chunk) {
                0.8
            } else {
                0.0
            }
        }
        WorkflowPhase::Review => {
            if is_test_like(chunk) {
                0.4
            } else {
                0.2
            }
        }
        _ => 0.0,
    }
}

fn query_mode_bias(query_mode: QueryMode, chunk: &ChunkRecord) -> f32 {
    match query_mode {
        QueryMode::UnderstandSymbol => {
            if chunk.chunk_kind == "Symbol" {
                0.6
            } else {
                0.0
            }
        }
        QueryMode::ImplementAdjacent => {
            if chunk.chunk_kind == "BodyRegion" {
                0.4
            } else {
                0.0
            }
        }
        QueryMode::BoundedRefactor | QueryMode::BlastRadius => {
            if is_test_like(chunk) {
                0.6
            } else {
                0.2
            }
        }
        QueryMode::FindExamples => {
            if is_example_like(chunk) {
                0.8
            } else {
                0.1
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

fn worktree_diff_bias(query_mode: QueryMode) -> f32 {
    match query_mode {
        QueryMode::BoundedRefactor | QueryMode::BlastRadius => 1.2,
        QueryMode::ImplementAdjacent => 0.8,
        QueryMode::FindExamples => 0.5,
        QueryMode::UnderstandSymbol => 0.4,
    }
}
