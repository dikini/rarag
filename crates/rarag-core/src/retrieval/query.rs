use crate::metadata::ChunkRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowPhase {
    Spec,
    Plan,
    WriteTests,
    WriteCode,
    Verify,
    Review,
    Fix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalRequest {
    pub snapshot_id: String,
    pub query_mode: QueryMode,
    pub workflow_phase: WorkflowPhase,
    pub query_text: String,
    pub symbol_path: Option<String>,
    pub limit: usize,
}

impl RetrievalRequest {
    pub fn new(
        snapshot_id: impl Into<String>,
        query_mode: QueryMode,
        workflow_phase: WorkflowPhase,
        query_text: impl Into<String>,
    ) -> Self {
        Self {
            snapshot_id: snapshot_id.into(),
            query_mode,
            workflow_phase,
            query_text: query_text.into(),
            symbol_path: None,
            limit: query_mode.neighborhood_cap(),
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

    pub fn effective_limit(&self) -> usize {
        self.limit.max(1).min(self.query_mode.neighborhood_cap())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrievedChunk {
    pub snapshot_id: String,
    pub chunk: ChunkRecord,
    pub score: f32,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalResponse {
    pub items: Vec<RetrievedChunk>,
    pub warnings: Vec<String>,
}
