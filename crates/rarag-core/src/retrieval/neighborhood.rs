use std::collections::HashMap;

use crate::metadata::{ChunkRecord, EdgeRecord};
use crate::retrieval::query::{QueryMode, RetrievalRequest};
use crate::retrieval::rerank::Candidate;

pub fn assemble_neighborhood(
    request: &RetrievalRequest,
    all_chunks: &[ChunkRecord],
    seed_chunks: &[ChunkRecord],
    all_edges: &[EdgeRecord],
) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let chunk_by_id: HashMap<_, _> = all_chunks
        .iter()
        .map(|chunk| (chunk.chunk_id.clone(), chunk))
        .collect();
    let symbol_path = request.symbol_path.as_deref();
    let symbol_leaf = symbol_path
        .and_then(|path| path.rsplit("::").next())
        .unwrap_or(request.query_text.as_str());

    for chunk in seed_chunks {
        candidates.push(Candidate {
            chunk: chunk.clone(),
            score: 10.0,
            evidence: vec!["exact_symbol".to_string()],
        });
    }

    for chunk in all_chunks {
        if seed_chunks
            .iter()
            .any(|seed| seed.chunk_id == chunk.chunk_id)
        {
            continue;
        }

        if same_file_neighbor(seed_chunks, chunk) {
            candidates.push(Candidate {
                chunk: chunk.clone(),
                score: 4.0,
                evidence: vec!["same_file".to_string()],
            });
        }

        if chunk.text.contains(symbol_leaf) {
            candidates.push(Candidate {
                chunk: chunk.clone(),
                score: text_reference_score(request.query_mode, chunk),
                evidence: vec!["text_reference".to_string()],
            });
        }

        if matches!(
            request.query_mode,
            QueryMode::FindExamples | QueryMode::BoundedRefactor
        ) && chunk.chunk_kind == "TestFunction"
        {
            candidates.push(Candidate {
                chunk: chunk.clone(),
                score: 3.5,
                evidence: vec!["test_neighbor".to_string()],
            });
        }

        if matches!(request.query_mode, QueryMode::UnderstandSymbol)
            && chunk.chunk_kind == "ModuleSummary"
            && symbol_path.is_some_and(|path| {
                chunk
                    .symbol_path
                    .as_deref()
                    .is_some_and(|candidate| path.starts_with(candidate))
            })
        {
            candidates.push(Candidate {
                chunk: chunk.clone(),
                score: 2.5,
                evidence: vec!["module_context".to_string()],
            });
        }
    }

    for edge in all_edges {
        let from_is_seed = seed_chunks
            .iter()
            .any(|seed| seed.chunk_id == edge.from_chunk_id);
        let to_is_seed = seed_chunks
            .iter()
            .any(|seed| seed.chunk_id == edge.to_chunk_id);
        if !from_is_seed && !to_is_seed {
            continue;
        }

        let neighbor_id = if from_is_seed {
            &edge.to_chunk_id
        } else {
            &edge.from_chunk_id
        };
        let Some(chunk) = chunk_by_id.get(neighbor_id) else {
            continue;
        };
        let Some((score, evidence)) = semantic_edge_score(request.query_mode, edge) else {
            continue;
        };
        candidates.push(Candidate {
            chunk: (*chunk).clone(),
            score,
            evidence: vec![evidence.to_string()],
        });
    }

    candidates
}

fn same_file_neighbor(seed_chunks: &[ChunkRecord], candidate: &ChunkRecord) -> bool {
    seed_chunks
        .iter()
        .any(|seed| seed.file_path == candidate.file_path && seed.chunk_id != candidate.chunk_id)
}

fn text_reference_score(query_mode: QueryMode, chunk: &ChunkRecord) -> f32 {
    match query_mode {
        QueryMode::BoundedRefactor | QueryMode::BlastRadius => {
            if chunk.chunk_kind == "TestFunction" {
                6.0
            } else {
                5.0
            }
        }
        QueryMode::FindExamples => 5.5,
        QueryMode::ImplementAdjacent => 4.5,
        QueryMode::UnderstandSymbol => 3.0,
    }
}

fn semantic_edge_score(query_mode: QueryMode, edge: &EdgeRecord) -> Option<(f32, &'static str)> {
    let kind = edge.semantic_kind()?;
    match kind {
        crate::semantic::SemanticEdgeKind::Reference => Some((
            match query_mode {
                QueryMode::BoundedRefactor | QueryMode::BlastRadius => 5.8,
                QueryMode::FindExamples => 5.2,
                QueryMode::ImplementAdjacent => 4.8,
                QueryMode::UnderstandSymbol => 3.8,
            },
            "semantic_reference",
        )),
        crate::semantic::SemanticEdgeKind::Implementation => Some((
            match query_mode {
                QueryMode::BoundedRefactor | QueryMode::BlastRadius => 8.6,
                QueryMode::ImplementAdjacent => 6.8,
                QueryMode::FindExamples => 4.0,
                QueryMode::UnderstandSymbol => 4.4,
            },
            "semantic_impl",
        )),
        crate::semantic::SemanticEdgeKind::Test => Some((
            match query_mode {
                QueryMode::BoundedRefactor | QueryMode::BlastRadius | QueryMode::FindExamples => {
                    8.2
                }
                QueryMode::ImplementAdjacent => 4.6,
                QueryMode::UnderstandSymbol => 3.6,
            },
            "semantic_test",
        )),
    }
}
