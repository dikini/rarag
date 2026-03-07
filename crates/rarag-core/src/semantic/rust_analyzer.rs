use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::chunking::{Chunk, ChunkKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticEdgeKind {
    Reference,
    Implementation,
    Test,
}

impl SemanticEdgeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Implementation => "implementation",
            Self::Test => "test",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "reference" => Some(Self::Reference),
            "implementation" => Some(Self::Implementation),
            "test" => Some(Self::Test),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEdge {
    pub edge_id: String,
    pub from_chunk_id: String,
    pub to_chunk_id: String,
    pub kind: SemanticEdgeKind,
    pub from_symbol_path: Option<String>,
    pub to_symbol_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEnrichment {
    pub edges: Vec<SemanticEdge>,
    pub warnings: Vec<String>,
    pub analyzer_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AnalyzerMode {
    Heuristic,
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustAnalyzerEnricher {
    mode: AnalyzerMode,
}

impl RustAnalyzerEnricher {
    pub fn heuristic() -> Self {
        Self {
            mode: AnalyzerMode::Heuristic,
        }
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            mode: AnalyzerMode::Unavailable(reason.into()),
        }
    }

    pub fn enrich_chunks(
        &self,
        _workspace_root: &Path,
        chunks: &[Chunk],
    ) -> Result<SemanticEnrichment, String> {
        match &self.mode {
            AnalyzerMode::Unavailable(reason) => Ok(SemanticEnrichment {
                edges: Vec::new(),
                warnings: vec![format!(
                    "rust-analyzer semantic enrichment unavailable: {reason}"
                )],
                analyzer_available: false,
            }),
            AnalyzerMode::Heuristic => Ok(SemanticEnrichment {
                edges: heuristic_edges(chunks),
                warnings: vec![
                    "rust-analyzer semantic enrichment unavailable; using heuristic fallback"
                        .to_string(),
                ],
                analyzer_available: false,
            }),
        }
    }
}

fn heuristic_edges(chunks: &[Chunk]) -> Vec<SemanticEdge> {
    let mut edges = Vec::new();
    let symbol_chunks: Vec<_> = chunks
        .iter()
        .filter(|chunk| chunk.kind == ChunkKind::Symbol)
        .collect();
    let mut seen = HashSet::new();

    for symbol in &symbol_chunks {
        let Some(symbol_path) = symbol.symbol_path.as_deref() else {
            continue;
        };
        let symbol_leaf = symbol_path.rsplit("::").next().unwrap_or(symbol_path);

        for chunk in chunks.iter().filter(|candidate| candidate.id != symbol.id) {
            if !contains_symbol_reference(chunk, symbol_leaf) {
                continue;
            }

            let kind = if chunk.kind == ChunkKind::TestFunction {
                SemanticEdgeKind::Test
            } else {
                SemanticEdgeKind::Reference
            };
            push_edge(
                &mut edges,
                &mut seen,
                chunk,
                symbol,
                kind,
                chunk.symbol_path.clone(),
                Some(symbol_path.to_string()),
            );
        }
    }

    let symbol_by_leaf: HashMap<_, _> = symbol_chunks
        .iter()
        .filter_map(|chunk| {
            let symbol_path = chunk.symbol_path.as_ref()?;
            if symbol_path.contains("::impl::") {
                return None;
            }
            Some((
                symbol_path
                    .rsplit("::")
                    .next()
                    .unwrap_or(symbol_path.as_str()),
                *chunk,
            ))
        })
        .collect();

    for chunk in chunks
        .iter()
        .filter(|chunk| chunk.kind == ChunkKind::Symbol)
    {
        let Some(symbol_path) = chunk.symbol_path.as_deref() else {
            continue;
        };
        if !symbol_path.contains("::impl") {
            continue;
        }

        if let Some(type_name) = impl_target_name(&chunk.text)
            && let Some(target) = symbol_by_leaf.get(type_name)
        {
            push_edge(
                &mut edges,
                &mut seen,
                chunk,
                target,
                SemanticEdgeKind::Implementation,
                Some(format!("{symbol_path}::{type_name}")),
                target.symbol_path.clone(),
            );
        }
    }

    edges
}

fn contains_symbol_reference(chunk: &Chunk, symbol_leaf: &str) -> bool {
    !matches!(
        chunk.kind,
        ChunkKind::CrateSummary | ChunkKind::ModuleSummary
    ) && chunk.text.contains(symbol_leaf)
}

fn impl_target_name(text: &str) -> Option<&str> {
    let impl_text = text.trim_start();
    let impl_text = impl_text.strip_prefix("impl ")?;
    let head = impl_text
        .split(|ch: char| ch == '{' || ch == '<' || ch.is_whitespace())
        .next()?;
    if head.is_empty() { None } else { Some(head) }
}

fn push_edge(
    edges: &mut Vec<SemanticEdge>,
    seen: &mut HashSet<String>,
    from: &Chunk,
    to: &Chunk,
    kind: SemanticEdgeKind,
    from_symbol_path: Option<String>,
    to_symbol_path: Option<String>,
) {
    let edge_id = format!("{}:{}:{}", from.id, to.id, kind.as_str());
    if seen.insert(edge_id.clone()) {
        edges.push(SemanticEdge {
            edge_id,
            from_chunk_id: from.id.clone(),
            to_chunk_id: to.id.clone(),
            kind,
            from_symbol_path,
            to_symbol_path,
        });
    }
}
