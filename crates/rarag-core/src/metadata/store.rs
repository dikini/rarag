use crate::metadata::{IndexingRunRecord, QueryAuditRecord, SnapshotRecord};
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

fn split_feature_set(feature_set: &str) -> Vec<&str> {
    feature_set
        .split(',')
        .filter(|feature| !feature.is_empty())
        .collect()
}
