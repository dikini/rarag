mod store;

use crate::config::{ObservabilityConfig, RetrievalConfig};
use crate::chunking::Chunk;
use crate::semantic::{SemanticEdge, SemanticEdgeKind};
use crate::snapshot::SnapshotKey;
use serde::{Deserialize, Serialize};

pub use store::SnapshotStore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub key: SnapshotKey,
    pub last_indexed_chunk_count: Option<u64>,
    pub last_query_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub chunk_id: String,
    pub snapshot_id: String,
    pub chunk_kind: String,
    pub symbol_path: Option<String>,
    pub symbol_name: Option<String>,
    pub owning_symbol_header: Option<String>,
    pub docs_text: Option<String>,
    pub signature_text: Option<String>,
    pub parent_symbol_path: Option<String>,
    pub retrieval_markers: Vec<String>,
    pub repository_state_hints: Vec<String>,
    pub file_path: String,
    pub start_byte: u32,
    pub end_byte: u32,
    pub text: String,
}

impl ChunkRecord {
    pub fn from_chunk(snapshot_id: impl Into<String>, chunk: &Chunk) -> Self {
        Self {
            chunk_id: chunk.id.clone(),
            snapshot_id: snapshot_id.into(),
            chunk_kind: format!("{:?}", chunk.kind),
            symbol_path: chunk.symbol_path.clone(),
            symbol_name: chunk.symbol_name.clone(),
            owning_symbol_header: chunk.owning_symbol_header.clone(),
            docs_text: chunk.docs_text.clone(),
            signature_text: chunk.signature_text.clone(),
            parent_symbol_path: chunk.parent_symbol_path.clone(),
            retrieval_markers: chunk.retrieval_markers.clone(),
            repository_state_hints: chunk.repository_state_hints.clone(),
            file_path: chunk.file_path.display().to_string(),
            start_byte: chunk.span.start_byte,
            end_byte: chunk.span.end_byte,
            text: chunk.text.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub edge_id: String,
    pub snapshot_id: String,
    pub from_chunk_id: String,
    pub to_chunk_id: String,
    pub edge_kind: String,
    pub from_symbol_path: Option<String>,
    pub to_symbol_path: Option<String>,
}

impl EdgeRecord {
    pub fn from_semantic_edge(snapshot_id: impl Into<String>, edge: &SemanticEdge) -> Self {
        Self {
            edge_id: edge.edge_id.clone(),
            snapshot_id: snapshot_id.into(),
            from_chunk_id: edge.from_chunk_id.clone(),
            to_chunk_id: edge.to_chunk_id.clone(),
            edge_kind: edge.kind.as_str().to_string(),
            from_symbol_path: edge.from_symbol_path.clone(),
            to_symbol_path: edge.to_symbol_path.clone(),
        }
    }

    pub fn semantic_kind(&self) -> Option<SemanticEdgeKind> {
        SemanticEdgeKind::parse(&self.edge_kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexingRunRecord {
    pub snapshot_id: String,
    pub status: String,
    pub chunk_count: u64,
}

impl IndexingRunRecord {
    pub fn new(
        snapshot_id: impl Into<String>,
        status: impl Into<String>,
        chunk_count: u64,
    ) -> Self {
        Self {
            snapshot_id: snapshot_id.into(),
            status: status.into(),
            chunk_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryAuditRecord {
    pub snapshot_id: String,
    pub query_mode: String,
    pub query_text: String,
    pub result_count: u64,
}

impl QueryAuditRecord {
    pub fn new(
        snapshot_id: impl Into<String>,
        query_mode: impl Into<String>,
        query_text: impl Into<String>,
        result_count: u64,
    ) -> Self {
        Self {
            snapshot_id: snapshot_id.into(),
            query_mode: query_mode.into(),
            query_text: query_text.into(),
            result_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryObservationRecord {
    pub observation_id: String,
    pub snapshot_id: String,
    pub query_mode: String,
    pub query_text: String,
    pub symbol_path: Option<String>,
    pub changed_paths: Vec<String>,
    pub warnings: Vec<String>,
    pub result_count: u64,
    pub retrieval: RetrievalConfig,
    pub observability: ObservabilityConfig,
}

impl QueryObservationRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        observation_id: impl Into<String>,
        snapshot_id: impl Into<String>,
        query_mode: impl Into<String>,
        query_text: impl Into<String>,
        symbol_path: Option<String>,
        changed_paths: Vec<String>,
        warnings: Vec<String>,
        result_count: u64,
        retrieval: RetrievalConfig,
        observability: ObservabilityConfig,
    ) -> Self {
        Self {
            observation_id: observation_id.into(),
            snapshot_id: snapshot_id.into(),
            query_mode: query_mode.into(),
            query_text: query_text.into(),
            symbol_path,
            changed_paths,
            warnings,
            result_count,
            retrieval,
            observability,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateObservationRecord {
    pub observation_id: String,
    pub chunk_id: String,
    pub chunk_kind: String,
    pub symbol_path: Option<String>,
    pub file_path: String,
    pub evidence: Vec<String>,
    pub retrieval_markers: Vec<String>,
    pub rank: u32,
    pub returned: bool,
    pub matched_worktree: bool,
    pub base_score: f32,
    pub query_mode_bias: f32,
    pub worktree_diff_bias: f32,
    pub final_score: f32,
}

impl CandidateObservationRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        observation_id: impl Into<String>,
        chunk_id: impl Into<String>,
        chunk_kind: impl Into<String>,
        symbol_path: Option<String>,
        file_path: impl Into<String>,
        evidence: Vec<String>,
        retrieval_markers: Vec<String>,
        rank: u32,
        returned: bool,
        matched_worktree: bool,
        base_score: f32,
        query_mode_bias: f32,
        worktree_diff_bias: f32,
        final_score: f32,
    ) -> Self {
        Self {
            observation_id: observation_id.into(),
            chunk_id: chunk_id.into(),
            chunk_kind: chunk_kind.into(),
            symbol_path,
            file_path: file_path.into(),
            evidence,
            retrieval_markers,
            rank,
            returned,
            matched_worktree,
            base_score,
            query_mode_bias,
            worktree_diff_bias,
            final_score,
        }
    }
}
