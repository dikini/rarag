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
    ReloadConfig,
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
    #[serde(default)]
    pub include_history: bool,
    pub history_max_nodes: Option<usize>,
    pub eval_task_id: Option<String>,
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
        if self.include_history {
            request = request.with_history(true);
        }
        if let Some(history_max_nodes) = self.history_max_nodes {
            request = request.with_history_max_nodes(history_max_nodes);
        }
        if let Some(eval_task_id) = self.eval_task_id {
            request = request.with_eval_task_id(eval_task_id);
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
    Reloaded(ReloadResponse),
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
pub struct ReloadResponse {
    pub generation: u64,
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
