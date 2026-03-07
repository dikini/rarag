use crate::chunking::Chunk;
use crate::metadata::{
    ChunkRecord, EdgeRecord, IndexingRunRecord, QueryAuditRecord, SnapshotRecord,
};
use crate::semantic::SemanticEdge;
use crate::snapshot::SnapshotKey;
use turso::{Builder, Connection, Database, Rows};

const SCHEMA_SQL: &str = include_str!("schema.sql");

#[derive(Debug)]
pub struct SnapshotStore {
    _database: Database,
    connection: Connection,
}

impl SnapshotStore {
    pub async fn open_local(path: &str) -> Result<Self, String> {
        let database = Builder::new_local(path)
            .build()
            .await
            .map_err(|err| err.to_string())?;
        let connection = database.connect().map_err(|err| err.to_string())?;
        connection
            .execute_batch(SCHEMA_SQL)
            .await
            .map_err(|err| err.to_string())?;

        Ok(Self {
            _database: database,
            connection,
        })
    }

    pub async fn create_or_get_snapshot(&self, key: SnapshotKey) -> Result<SnapshotRecord, String> {
        let snapshot_id = key.id();
        let feature_set = key.feature_set.join(",");

        self.connection
            .execute(
                "INSERT OR IGNORE INTO snapshots (snapshot_id, repo_root, worktree_root, git_sha, cargo_target, feature_set, cfg_profile) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    snapshot_id.as_str(),
                    key.repo_root.as_str(),
                    key.worktree_root.as_str(),
                    key.git_sha.as_str(),
                    key.cargo_target.as_str(),
                    feature_set.as_str(),
                    key.cfg_profile.as_str(),
                ),
            )
            .await
            .map_err(|err| err.to_string())?;

        self.load_snapshot(&snapshot_id)
            .await?
            .ok_or_else(|| format!("snapshot {snapshot_id} missing after insert"))
    }

    pub async fn load_snapshot(&self, snapshot_id: &str) -> Result<Option<SnapshotRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT snapshot_id, repo_root, worktree_root, git_sha, cargo_target, feature_set, cfg_profile FROM snapshots WHERE snapshot_id = ?1",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;

        let Some(row) = rows.next().await.map_err(|err| err.to_string())? else {
            return Ok(None);
        };

        let key = SnapshotKey::new(
            text_at(&row, 1)?,
            text_at(&row, 2)?,
            text_at(&row, 3)?,
            text_at(&row, 4)?,
            split_feature_set(&text_at(&row, 5)?),
            text_at(&row, 6)?,
        );

        Ok(Some(SnapshotRecord {
            id: text_at(&row, 0)?,
            key,
            last_indexed_chunk_count: self.latest_chunk_count(snapshot_id).await?,
            last_query_mode: self.latest_query_mode(snapshot_id).await?,
        }))
    }

    pub async fn record_indexing_run(&self, record: IndexingRunRecord) -> Result<(), String> {
        self.connection
            .execute(
                "INSERT INTO indexing_runs (snapshot_id, status, chunk_count) VALUES (?1, ?2, ?3)",
                (
                    record.snapshot_id.as_str(),
                    record.status.as_str(),
                    i64::try_from(record.chunk_count).map_err(|err| err.to_string())?,
                ),
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn load_chunks(&self, snapshot_id: &str) -> Result<Vec<ChunkRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT chunk_id, snapshot_id, chunk_kind, symbol_path, file_path, start_byte, end_byte, text FROM chunks WHERE snapshot_id = ?1 ORDER BY file_path, start_byte",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;

        let mut chunks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            chunks.push(ChunkRecord {
                chunk_id: text_at(&row, 0)?,
                snapshot_id: text_at(&row, 1)?,
                chunk_kind: text_at(&row, 2)?,
                symbol_path: optional_text_at(&row, 3)?,
                file_path: text_at(&row, 4)?,
                start_byte: u32_at(&row, 5)?,
                end_byte: u32_at(&row, 6)?,
                text: text_at(&row, 7)?,
            });
        }

        Ok(chunks)
    }

    pub async fn load_edges(&self, snapshot_id: &str) -> Result<Vec<EdgeRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT edge_id, snapshot_id, from_chunk_id, to_chunk_id, edge_kind, from_symbol_path, to_symbol_path FROM edges WHERE snapshot_id = ?1 ORDER BY edge_kind, from_chunk_id, to_chunk_id",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;

        let mut edges = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            edges.push(EdgeRecord {
                edge_id: text_at(&row, 0)?,
                snapshot_id: text_at(&row, 1)?,
                from_chunk_id: text_at(&row, 2)?,
                to_chunk_id: text_at(&row, 3)?,
                edge_kind: text_at(&row, 4)?,
                from_symbol_path: optional_text_at(&row, 5)?,
                to_symbol_path: optional_text_at(&row, 6)?,
            });
        }

        Ok(edges)
    }

    pub async fn record_query_audit(&self, record: QueryAuditRecord) -> Result<(), String> {
        self.connection
            .execute(
                "INSERT INTO query_audits (snapshot_id, query_mode, query_text, result_count) VALUES (?1, ?2, ?3, ?4)",
                (
                    record.snapshot_id.as_str(),
                    record.query_mode.as_str(),
                    record.query_text.as_str(),
                    i64::try_from(record.result_count).map_err(|err| err.to_string())?,
                ),
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub async fn replace_chunks(&self, snapshot_id: &str, chunks: &[Chunk]) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM chunks WHERE snapshot_id = ?1", [snapshot_id])
            .await
            .map_err(|err| err.to_string())?;

        for record in chunks
            .iter()
            .map(|chunk| ChunkRecord::from_chunk(snapshot_id.to_string(), chunk))
        {
            self.connection
                .execute(
                    "INSERT INTO chunks (chunk_id, snapshot_id, chunk_kind, symbol_path, file_path, start_byte, end_byte, text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    (
                        record.chunk_id.as_str(),
                        record.snapshot_id.as_str(),
                        record.chunk_kind.as_str(),
                        record.symbol_path.as_deref(),
                        record.file_path.as_str(),
                        i64::from(record.start_byte),
                        i64::from(record.end_byte),
                        record.text.as_str(),
                    ),
                )
                .await
                .map_err(|err| err.to_string())?;
        }

        Ok(())
    }

    pub async fn replace_edges(
        &self,
        snapshot_id: &str,
        edges: &[SemanticEdge],
    ) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM edges WHERE snapshot_id = ?1", [snapshot_id])
            .await
            .map_err(|err| err.to_string())?;

        for record in edges
            .iter()
            .map(|edge| EdgeRecord::from_semantic_edge(snapshot_id.to_string(), edge))
        {
            self.connection
                .execute(
                    "INSERT INTO edges (edge_id, snapshot_id, from_chunk_id, to_chunk_id, edge_kind, from_symbol_path, to_symbol_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    (
                        record.edge_id.as_str(),
                        record.snapshot_id.as_str(),
                        record.from_chunk_id.as_str(),
                        record.to_chunk_id.as_str(),
                        record.edge_kind.as_str(),
                        record.from_symbol_path.as_deref(),
                        record.to_symbol_path.as_deref(),
                    ),
                )
                .await
                .map_err(|err| err.to_string())?;
        }

        Ok(())
    }

    pub async fn chunk_count(&self, snapshot_id: &str) -> Result<usize, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT COUNT(*) FROM chunks WHERE snapshot_id = ?1",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;

        match rows.next().await.map_err(|err| err.to_string())? {
            Some(row) => {
                let count: i64 = row.get(0).map_err(|err| err.to_string())?;
                usize::try_from(count).map_err(|err| err.to_string())
            }
            None => Ok(0),
        }
    }

    async fn latest_chunk_count(&self, snapshot_id: &str) -> Result<Option<u64>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT chunk_count FROM indexing_runs WHERE snapshot_id = ?1 ORDER BY indexing_run_id DESC LIMIT 1",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        first_optional_u64(&mut rows).await
    }

    async fn latest_query_mode(&self, snapshot_id: &str) -> Result<Option<String>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT query_mode FROM query_audits WHERE snapshot_id = ?1 ORDER BY query_audit_id DESC LIMIT 1",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        first_optional_text(&mut rows).await
    }
}

async fn first_optional_u64(rows: &mut Rows) -> Result<Option<u64>, String> {
    match rows.next().await.map_err(|err| err.to_string())? {
        Some(row) => {
            let value: i64 = row.get(0).map_err(|err| err.to_string())?;
            u64::try_from(value)
                .map(Some)
                .map_err(|err| err.to_string())
        }
        None => Ok(None),
    }
}

async fn first_optional_text(rows: &mut Rows) -> Result<Option<String>, String> {
    match rows.next().await.map_err(|err| err.to_string())? {
        Some(row) => Ok(Some(text_at(&row, 0)?)),
        None => Ok(None),
    }
}

fn text_at(row: &turso::Row, index: usize) -> Result<String, String> {
    row.get(index).map_err(|err| err.to_string())
}

fn optional_text_at(row: &turso::Row, index: usize) -> Result<Option<String>, String> {
    row.get(index).map_err(|err| err.to_string())
}

fn u32_at(row: &turso::Row, index: usize) -> Result<u32, String> {
    let value: i64 = row.get(index).map_err(|err| err.to_string())?;
    u32::try_from(value).map_err(|err| err.to_string())
}

fn split_feature_set(feature_set: &str) -> Vec<&str> {
    feature_set
        .split(',')
        .filter(|feature| !feature.is_empty())
        .collect()
}
