mod store;

use crate::snapshot::SnapshotKey;

pub use store::SnapshotStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotRecord {
    pub id: String,
    pub key: SnapshotKey,
    pub last_indexed_chunk_count: Option<u64>,
    pub last_query_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
