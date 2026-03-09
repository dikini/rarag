use crate::metadata::ChunkRecord;
use crate::worktree::WorktreeChanges;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryMode {
    UnderstandSymbol,
    ImplementAdjacent,
    BoundedRefactor,
    FindExamples,
    BlastRadius,
}

impl QueryMode {
    pub const fn neighborhood_cap(self) -> usize {
        match self {
            Self::UnderstandSymbol => 4,
            Self::ImplementAdjacent => 6,
            Self::BoundedRefactor => 8,
            Self::FindExamples => 6,
            Self::BlastRadius => 10,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnderstandSymbol => "understand-symbol",
            Self::ImplementAdjacent => "implement-adjacent",
            Self::BoundedRefactor => "bounded-refactor",
            Self::FindExamples => "find-examples",
            Self::BlastRadius => "blast-radius",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub snapshot_id: String,
    pub query_mode: QueryMode,
    pub query_text: String,
    pub symbol_path: Option<String>,
    pub limit: usize,
    pub worktree_changes: WorktreeChanges,
    pub include_history: bool,
    pub history_max_nodes: Option<usize>,
    pub eval_task_id: Option<String>,
}

impl RetrievalRequest {
    pub fn new(
        snapshot_id: impl Into<String>,
        query_mode: QueryMode,
        query_text: impl Into<String>,
    ) -> Self {
        Self {
            snapshot_id: snapshot_id.into(),
            query_mode,
            query_text: query_text.into(),
            symbol_path: None,
            limit: query_mode.neighborhood_cap(),
            worktree_changes: WorktreeChanges::default(),
            include_history: false,
            history_max_nodes: None,
            eval_task_id: None,
        }
    }

    pub fn with_symbol_path(mut self, symbol_path: impl Into<String>) -> Self {
        self.symbol_path = Some(symbol_path.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_worktree_changes(mut self, worktree_changes: WorktreeChanges) -> Self {
        self.worktree_changes = worktree_changes;
        self
    }

    pub fn with_history(mut self, include_history: bool) -> Self {
        self.include_history = include_history;
        self
    }

    pub fn with_history_max_nodes(mut self, history_max_nodes: usize) -> Self {
        self.history_max_nodes = Some(history_max_nodes.max(1));
        self
    }

    pub fn with_eval_task_id(mut self, eval_task_id: impl Into<String>) -> Self {
        self.eval_task_id = Some(eval_task_id.into());
        self
    }

    pub fn effective_limit(&self) -> usize {
        self.limit.max(1).min(self.query_mode.neighborhood_cap())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievedChunk {
    pub snapshot_id: String,
    pub chunk: ChunkRecord,
    pub score: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalResponse {
    pub items: Vec<RetrievedChunk>,
    pub warnings: Vec<String>,
}
