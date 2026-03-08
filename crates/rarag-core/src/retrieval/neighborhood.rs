use std::collections::HashMap;

use crate::config::NeighborhoodWeightsConfig;
use crate::metadata::{ChunkRecord, EdgeRecord};
use crate::retrieval::query::{QueryMode, RetrievalRequest};
use crate::retrieval::rerank::Candidate;

pub fn assemble_neighborhood(
    request: &RetrievalRequest,
    weights: &NeighborhoodWeightsConfig,
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
            score: weights.exact_symbol,
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
                score: weights.same_file,
                evidence: vec!["same_file".to_string()],
            });
        }

        if chunk.text.contains(symbol_leaf) {
            candidates.push(Candidate {
                chunk: chunk.clone(),
                score: text_reference_score(request.query_mode, chunk, weights),
                evidence: vec!["text_reference".to_string()],
            });
        }

        if matches!(
            request.query_mode,
            QueryMode::FindExamples | QueryMode::BoundedRefactor
        ) && is_example_or_test_chunk(chunk)
        {
            candidates.push(Candidate {
                chunk: chunk.clone(),
                score: match request.query_mode {
                    QueryMode::FindExamples => weights.test_neighbor_find_examples,
                    QueryMode::BoundedRefactor => weights.test_neighbor_bounded_refactor,
                    _ => unreachable!("guarded by query mode match"),
                },
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
                score: weights.module_context_understand_symbol,
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
        let Some((score, evidence)) = semantic_edge_score(request.query_mode, edge, weights) else {
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

#[cfg(test)]
mod tests {
    use super::assemble_neighborhood;
    use crate::config::NeighborhoodWeightsConfig;
    use crate::metadata::{ChunkRecord, EdgeRecord};
    use crate::retrieval::{QueryMode, RetrievalRequest};

    fn chunk(chunk_id: &str, chunk_kind: &str, file_path: &str) -> ChunkRecord {
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
            retrieval_markers: Vec::new(),
            repository_state_hints: Vec::new(),
            file_path: file_path.to_string(),
            start_byte: 0,
            end_byte: 10,
            text: chunk_id.to_string(),
        }
    }

    fn semantic_edge(from_chunk_id: &str, to_chunk_id: &str, edge_kind: &str) -> EdgeRecord {
        EdgeRecord {
            edge_id: format!("{from_chunk_id}-{to_chunk_id}-{edge_kind}"),
            snapshot_id: "snapshot-1".to_string(),
            from_chunk_id: from_chunk_id.to_string(),
            to_chunk_id: to_chunk_id.to_string(),
            edge_kind: edge_kind.to_string(),
            from_symbol_path: Some(format!("mini_repo::{from_chunk_id}")),
            to_symbol_path: Some(format!("mini_repo::{to_chunk_id}")),
        }
    }

    #[test]
    fn default_weights_preserve_exact_symbol_seed_score() {
        let seed = chunk("seed", "Symbol", "src/lib.rs");
        let request = RetrievalRequest::new("snapshot-1", QueryMode::UnderstandSymbol, "seed")
            .with_symbol_path("mini_repo::seed");

        let candidates = assemble_neighborhood(
            &request,
            &NeighborhoodWeightsConfig::default(),
            &[seed.clone()],
            &[seed],
            &[],
        );

        assert_eq!(candidates[0].score, 10.0);
        assert!(candidates[0].evidence.iter().any(|item| item == "exact_symbol"));
    }

    #[test]
    fn override_semantic_impl_weight_changes_candidate_score() {
        let seed = chunk("seed", "Symbol", "src/lib.rs");
        let neighbor = chunk("impl", "Symbol", "src/impl.rs");
        let request = RetrievalRequest::new("snapshot-1", QueryMode::BoundedRefactor, "seed")
            .with_symbol_path("mini_repo::seed");
        let mut weights = NeighborhoodWeightsConfig::default();
        weights.semantic_impl_bounded_refactor = 42.0;

        let candidates = assemble_neighborhood(
            &request,
            &weights,
            &[seed.clone(), neighbor.clone()],
            &[seed],
            &[semantic_edge("seed", "impl", "implementation")],
        );

        let candidate = candidates
            .iter()
            .find(|candidate| candidate.chunk.chunk_id == neighbor.chunk_id)
            .expect("semantic impl candidate");
        assert_eq!(candidate.score, 42.0);
        assert!(candidate.evidence.iter().any(|item| item == "semantic_impl"));
    }
}

fn is_example_or_test_chunk(chunk: &ChunkRecord) -> bool {
    matches!(
        chunk.chunk_kind.as_str(),
        "TestFunction" | "ExampleFile" | "Doctest"
    ) || chunk
        .retrieval_markers
        .iter()
        .any(|marker| matches!(marker.as_str(), "test" | "example" | "doctest"))
}

fn same_file_neighbor(seed_chunks: &[ChunkRecord], candidate: &ChunkRecord) -> bool {
    seed_chunks
        .iter()
        .any(|seed| seed.file_path == candidate.file_path && seed.chunk_id != candidate.chunk_id)
}

fn text_reference_score(
    query_mode: QueryMode,
    chunk: &ChunkRecord,
    weights: &NeighborhoodWeightsConfig,
) -> f32 {
    match query_mode {
        QueryMode::BoundedRefactor => {
            if is_example_or_test_chunk(chunk) {
                weights.text_reference_bounded_refactor_test_like
            } else {
                weights.text_reference_bounded_refactor
            }
        }
        QueryMode::BlastRadius => {
            if is_example_or_test_chunk(chunk) {
                weights.text_reference_blast_radius_test_like
            } else {
                weights.text_reference_blast_radius
            }
        }
        QueryMode::FindExamples => {
            if is_example_or_test_chunk(chunk) {
                weights.text_reference_find_examples_test_like
            } else {
                weights.text_reference_find_examples
            }
        }
        QueryMode::ImplementAdjacent => weights.text_reference_implement_adjacent,
        QueryMode::UnderstandSymbol => weights.text_reference_understand_symbol,
    }
}

fn semantic_edge_score(
    query_mode: QueryMode,
    edge: &EdgeRecord,
    weights: &NeighborhoodWeightsConfig,
) -> Option<(f32, &'static str)> {
    let kind = edge.semantic_kind()?;
    match kind {
        crate::semantic::SemanticEdgeKind::Reference => Some((
            match query_mode {
                QueryMode::BoundedRefactor => weights.semantic_reference_bounded_refactor,
                QueryMode::BlastRadius => weights.semantic_reference_blast_radius,
                QueryMode::FindExamples => weights.semantic_reference_find_examples,
                QueryMode::ImplementAdjacent => weights.semantic_reference_implement_adjacent,
                QueryMode::UnderstandSymbol => weights.semantic_reference_understand_symbol,
            },
            "semantic_reference",
        )),
        crate::semantic::SemanticEdgeKind::Implementation => Some((
            match query_mode {
                QueryMode::BoundedRefactor => weights.semantic_impl_bounded_refactor,
                QueryMode::BlastRadius => weights.semantic_impl_blast_radius,
                QueryMode::ImplementAdjacent => weights.semantic_impl_implement_adjacent,
                QueryMode::FindExamples => weights.semantic_impl_find_examples,
                QueryMode::UnderstandSymbol => weights.semantic_impl_understand_symbol,
            },
            "semantic_impl",
        )),
        crate::semantic::SemanticEdgeKind::Test => Some((
            match query_mode {
                QueryMode::BoundedRefactor => weights.semantic_test_bounded_refactor,
                QueryMode::BlastRadius => weights.semantic_test_blast_radius,
                QueryMode::FindExamples => weights.semantic_test_find_examples,
                QueryMode::ImplementAdjacent => weights.semantic_test_implement_adjacent,
                QueryMode::UnderstandSymbol => weights.semantic_test_understand_symbol,
            },
            "semantic_test",
        )),
    }
}
