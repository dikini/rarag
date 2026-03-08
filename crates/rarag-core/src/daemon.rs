use serde::{Deserialize, Serialize};

use crate::retrieval::{QueryMode, RetrievalRequest, RetrievalResponse};
use crate::snapshot::SnapshotKey;
use crate::worktree::WorktreeChanges;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DaemonRequest {
    Status {
        snapshot_id: Option<String>,
        worktree_root: Option<String>,
    },
    IndexWorkspace {
        snapshot: SnapshotKey,
        workspace_root: String,
        max_body_bytes: usize,
    },
    Query(QueryPayload),
    BlastRadius(QueryPayload),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryPayload {
    pub snapshot_id: Option<String>,
    pub worktree_root: Option<String>,
    pub query_mode: QueryMode,
    pub query_text: String,
    pub symbol_path: Option<String>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub changed_paths: Vec<String>,
}

impl QueryPayload {
    pub fn validate_locator(&self) -> Result<(), String> {
        if self.snapshot_id.is_some() || self.worktree_root.is_some() {
            Ok(())
        } else {
            Err("query requests require snapshot_id or worktree_root".to_string())
        }
    }

    pub fn into_retrieval_request(self, snapshot_id: String) -> RetrievalRequest {
        let mut request = RetrievalRequest::new(snapshot_id, self.query_mode, self.query_text);
        if let Some(symbol_path) = self.symbol_path {
            request = request.with_symbol_path(symbol_path);
        }
        if let Some(limit) = self.limit {
            request = request.with_limit(limit);
        }
        if !self.changed_paths.is_empty() {
            request =
                request.with_worktree_changes(WorktreeChanges::from_paths(self.changed_paths));
        }
        request
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DaemonResponse {
    Status(StatusPayload),
    Indexed(IndexResponse),
    Query(RetrievalResponse),
    Ack,
    Error(ErrorResponse),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusPayload {
    pub resolved_snapshot_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexResponse {
    pub snapshot_id: String,
    pub chunk_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
