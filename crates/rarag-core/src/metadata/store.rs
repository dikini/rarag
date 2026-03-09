use crate::chunking::Chunk;
use crate::metadata::{
    CandidateObservationRecord, ChunkRecord, DocumentBlockRecord, EdgeRecord, HistoryNodeRecord,
    IndexingRunRecord, LineageEdgeRecord, QueryAuditRecord, QueryObservationRecord,
    SnapshotRecord,
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

    pub async fn resolve_snapshot_for_worktree_root(
        &self,
        worktree_root: &str,
    ) -> Result<Option<SnapshotRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT snapshot_id FROM snapshots WHERE worktree_root = ?1 ORDER BY created_at DESC, snapshot_id DESC LIMIT 1",
                [worktree_root],
            )
            .await
            .map_err(|err| err.to_string())?;

        match rows.next().await.map_err(|err| err.to_string())? {
            Some(row) => self.load_snapshot(&text_at(&row, 0)?).await,
            None => Ok(None),
        }
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
                "SELECT chunk_id, snapshot_id, chunk_kind, symbol_path, symbol_name, owning_symbol_header, docs_text, signature_text, parent_symbol_path, retrieval_markers, repository_state_hints, file_path, start_byte, end_byte, text FROM chunks WHERE snapshot_id = ?1 ORDER BY file_path, start_byte",
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
                symbol_name: optional_text_at(&row, 4)?,
                owning_symbol_header: optional_text_at(&row, 5)?,
                docs_text: optional_text_at(&row, 6)?,
                signature_text: optional_text_at(&row, 7)?,
                parent_symbol_path: optional_text_at(&row, 8)?,
                retrieval_markers: split_csv(&text_at(&row, 9)?),
                repository_state_hints: split_csv(&text_at(&row, 10)?),
                file_path: text_at(&row, 11)?,
                start_byte: u32_at(&row, 12)?,
                end_byte: u32_at(&row, 13)?,
                text: text_at(&row, 14)?,
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

    pub async fn replace_document_blocks(
        &self,
        snapshot_id: &str,
        blocks: &[DocumentBlockRecord],
    ) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM document_blocks WHERE snapshot_id = ?1", [snapshot_id])
            .await
            .map_err(|err| err.to_string())?;
        for block in blocks {
            let heading_path =
                encode_string_list(&block.heading_path).map_err(|err| err.to_string())?;
            self.connection
                .execute(
                    "INSERT INTO document_blocks (block_id, snapshot_id, file_path, document_kind, parser, heading_path, start_line, end_line, text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    (
                        block.block_id.as_str(),
                        snapshot_id,
                        block.file_path.as_str(),
                        block.document_kind.as_str(),
                        block.parser.as_str(),
                        heading_path.as_str(),
                        i64::from(block.start_line),
                        i64::from(block.end_line),
                        block.text.as_str(),
                    ),
                )
                .await
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    pub async fn load_document_blocks(
        &self,
        snapshot_id: &str,
    ) -> Result<Vec<DocumentBlockRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT block_id, snapshot_id, file_path, document_kind, parser, heading_path, start_line, end_line, text FROM document_blocks WHERE snapshot_id = ?1 ORDER BY file_path, start_line, block_id",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        let mut blocks = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            blocks.push(DocumentBlockRecord {
                block_id: text_at(&row, 0)?,
                snapshot_id: text_at(&row, 1)?,
                file_path: text_at(&row, 2)?,
                document_kind: text_at(&row, 3)?,
                parser: text_at(&row, 4)?,
                heading_path: decode_string_list(&text_at(&row, 5)?)
                    .map_err(|err| err.to_string())?,
                start_line: u32_at(&row, 6)?,
                end_line: u32_at(&row, 7)?,
                text: text_at(&row, 8)?,
            });
        }
        Ok(blocks)
    }

    pub async fn replace_history_nodes(
        &self,
        snapshot_id: &str,
        nodes: &[HistoryNodeRecord],
    ) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM history_nodes WHERE snapshot_id = ?1", [snapshot_id])
            .await
            .map_err(|err| err.to_string())?;
        for node in nodes {
            self.connection
                .execute(
                    "INSERT INTO history_nodes (node_id, snapshot_id, node_kind, subject, summary) VALUES (?1, ?2, ?3, ?4, ?5)",
                    (
                        node.node_id.as_str(),
                        snapshot_id,
                        node.node_kind.as_str(),
                        node.subject.as_deref(),
                        node.summary.as_str(),
                    ),
                )
                .await
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    pub async fn load_history_nodes(
        &self,
        snapshot_id: &str,
    ) -> Result<Vec<HistoryNodeRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT node_id, snapshot_id, node_kind, subject, summary FROM history_nodes WHERE snapshot_id = ?1 ORDER BY node_kind, node_id",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        let mut nodes = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            nodes.push(HistoryNodeRecord {
                node_id: text_at(&row, 0)?,
                snapshot_id: text_at(&row, 1)?,
                node_kind: text_at(&row, 2)?,
                subject: optional_text_at(&row, 3)?,
                summary: text_at(&row, 4)?,
            });
        }
        Ok(nodes)
    }

    pub async fn replace_lineage_edges(
        &self,
        snapshot_id: &str,
        edges: &[LineageEdgeRecord],
    ) -> Result<(), String> {
        self.connection
            .execute("DELETE FROM lineage_edges WHERE snapshot_id = ?1", [snapshot_id])
            .await
            .map_err(|err| err.to_string())?;
        for edge in edges {
            self.connection
                .execute(
                    "INSERT INTO lineage_edges (edge_id, snapshot_id, from_node_id, to_node_id, edge_kind, evidence, confidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    (
                        edge.edge_id.as_str(),
                        snapshot_id,
                        edge.from_node_id.as_str(),
                        edge.to_node_id.as_str(),
                        edge.edge_kind.as_str(),
                        edge.evidence.as_deref(),
                        f64::from(edge.confidence),
                    ),
                )
                .await
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    pub async fn load_lineage_edges(
        &self,
        snapshot_id: &str,
    ) -> Result<Vec<LineageEdgeRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT edge_id, snapshot_id, from_node_id, to_node_id, edge_kind, evidence, confidence FROM lineage_edges WHERE snapshot_id = ?1 ORDER BY edge_kind, edge_id",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        let mut edges = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            edges.push(LineageEdgeRecord {
                edge_id: text_at(&row, 0)?,
                snapshot_id: text_at(&row, 1)?,
                from_node_id: text_at(&row, 2)?,
                to_node_id: text_at(&row, 3)?,
                edge_kind: text_at(&row, 4)?,
                evidence: optional_text_at(&row, 5)?,
                confidence: f32_at(&row, 6)?,
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

    pub async fn record_query_observation(
        &self,
        record: QueryObservationRecord,
        candidates: &[CandidateObservationRecord],
    ) -> Result<(), String> {
        let retrieval_json =
            serde_json::to_string(&record.retrieval).map_err(|err| err.to_string())?;
        let changed_paths_json =
            encode_string_list(&record.changed_paths).map_err(|err| err.to_string())?;
        let warnings_json = encode_string_list(&record.warnings).map_err(|err| err.to_string())?;
        let evidence_coverage_json = encode_string_list(&record.evidence_class_coverage)
            .map_err(|err| err.to_string())?;
        let tx = self
            .connection
            .unchecked_transaction()
            .await
            .map_err(|err| err.to_string())?;
        tx.execute(
                "INSERT INTO query_observations (observation_id, snapshot_id, query_mode, query_text, symbol_path, changed_paths, warnings, result_count, eval_task_id, evidence_class_coverage, retrieval_config_json, observability_enabled, observability_verbosity) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                (
                    record.observation_id.as_str(),
                    record.snapshot_id.as_str(),
                    record.query_mode.as_str(),
                    record.query_text.as_str(),
                    record.symbol_path.as_deref(),
                    changed_paths_json.as_str(),
                    warnings_json.as_str(),
                    i64::try_from(record.result_count).map_err(|err| err.to_string())?,
                    record.eval_task_id.as_deref(),
                    evidence_coverage_json.as_str(),
                    retrieval_json.as_str(),
                    if record.observability.enabled { 1_i64 } else { 0_i64 },
                    record.observability.verbosity.to_string(),
                ),
            )
            .await
            .map_err(|err| err.to_string())?;

        for candidate in candidates {
            let evidence_json =
                encode_string_list(&candidate.evidence).map_err(|err| err.to_string())?;
            let retrieval_markers_json =
                encode_string_list(&candidate.retrieval_markers).map_err(|err| err.to_string())?;
            tx.execute(
                    "INSERT INTO candidate_observations (observation_id, rank, chunk_id, chunk_kind, symbol_path, file_path, evidence, retrieval_markers, returned, matched_worktree, base_score, query_mode_bias, worktree_diff_bias, final_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    (
                        candidate.observation_id.as_str(),
                        i64::from(candidate.rank),
                        candidate.chunk_id.as_str(),
                        candidate.chunk_kind.as_str(),
                        candidate.symbol_path.as_deref(),
                        candidate.file_path.as_str(),
                        evidence_json.as_str(),
                        retrieval_markers_json.as_str(),
                        if candidate.returned { 1_i64 } else { 0_i64 },
                        if candidate.matched_worktree { 1_i64 } else { 0_i64 },
                        f64::from(candidate.base_score),
                        f64::from(candidate.query_mode_bias),
                        f64::from(candidate.worktree_diff_bias),
                        f64::from(candidate.final_score),
                    ),
                )
                .await
                .map_err(|err| err.to_string())?;
        }

        tx.commit().await.map_err(|err| err.to_string())?;

        Ok(())
    }

    pub async fn load_query_observations(
        &self,
        snapshot_id: &str,
    ) -> Result<Vec<QueryObservationRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT observation_id, snapshot_id, query_mode, query_text, symbol_path, changed_paths, warnings, result_count, eval_task_id, evidence_class_coverage, retrieval_config_json, observability_enabled, observability_verbosity FROM query_observations WHERE snapshot_id = ?1 ORDER BY recorded_at, observation_id",
                [snapshot_id],
            )
            .await
            .map_err(|err| err.to_string())?;

        let mut observations = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            let retrieval_json = text_at(&row, 10)?;
            let retrieval = serde_json::from_str(&retrieval_json).map_err(|err| err.to_string())?;
            let enabled: i64 = row.get(11).map_err(|err| err.to_string())?;
            observations.push(QueryObservationRecord {
                observation_id: text_at(&row, 0)?,
                snapshot_id: text_at(&row, 1)?,
                query_mode: text_at(&row, 2)?,
                query_text: text_at(&row, 3)?,
                symbol_path: optional_text_at(&row, 4)?,
                changed_paths: decode_string_list(&text_at(&row, 5)?)
                    .map_err(|err| err.to_string())?,
                warnings: decode_string_list(&text_at(&row, 6)?).map_err(|err| err.to_string())?,
                result_count: u64_at(&row, 7)?,
                eval_task_id: optional_text_at(&row, 8)?,
                evidence_class_coverage: decode_string_list(&text_at(&row, 9)?)
                    .map_err(|err| err.to_string())?,
                retrieval,
                observability: crate::config::ObservabilityConfig {
                    enabled: enabled != 0,
                    verbosity: text_at(&row, 12)?.parse().map_err(|err: String| err)?,
                },
            });
        }

        Ok(observations)
    }

    pub async fn load_candidate_observations(
        &self,
        observation_id: &str,
    ) -> Result<Vec<CandidateObservationRecord>, String> {
        let mut rows = self
            .connection
            .query(
                "SELECT observation_id, chunk_id, chunk_kind, symbol_path, file_path, evidence, retrieval_markers, rank, returned, matched_worktree, base_score, query_mode_bias, worktree_diff_bias, final_score FROM candidate_observations WHERE observation_id = ?1 ORDER BY rank, chunk_id",
                [observation_id],
            )
            .await
            .map_err(|err| err.to_string())?;

        let mut candidates = Vec::new();
        while let Some(row) = rows.next().await.map_err(|err| err.to_string())? {
            let returned: i64 = row.get(8).map_err(|err| err.to_string())?;
            let matched_worktree: i64 = row.get(9).map_err(|err| err.to_string())?;
            candidates.push(CandidateObservationRecord {
                observation_id: text_at(&row, 0)?,
                chunk_id: text_at(&row, 1)?,
                chunk_kind: text_at(&row, 2)?,
                symbol_path: optional_text_at(&row, 3)?,
                file_path: text_at(&row, 4)?,
                evidence: decode_string_list(&text_at(&row, 5)?).map_err(|err| err.to_string())?,
                retrieval_markers: decode_string_list(&text_at(&row, 6)?)
                    .map_err(|err| err.to_string())?,
                rank: u32_at(&row, 7)?,
                returned: returned != 0,
                matched_worktree: matched_worktree != 0,
                base_score: f32_at(&row, 10)?,
                query_mode_bias: f32_at(&row, 11)?,
                worktree_diff_bias: f32_at(&row, 12)?,
                final_score: f32_at(&row, 13)?,
            });
        }

        Ok(candidates)
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
                    "INSERT INTO chunks (chunk_id, snapshot_id, chunk_kind, symbol_path, symbol_name, owning_symbol_header, docs_text, signature_text, parent_symbol_path, retrieval_markers, repository_state_hints, file_path, start_byte, end_byte, text) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    (
                        record.chunk_id.as_str(),
                        record.snapshot_id.as_str(),
                        record.chunk_kind.as_str(),
                        record.symbol_path.as_deref(),
                        record.symbol_name.as_deref(),
                        record.owning_symbol_header.as_deref(),
                        record.docs_text.as_deref(),
                        record.signature_text.as_deref(),
                        record.parent_symbol_path.as_deref(),
                        join_csv(&record.retrieval_markers),
                        join_csv(&record.repository_state_hints),
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

fn join_csv(values: &[String]) -> String {
    values.join(",")
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn encode_string_list(values: &[String]) -> Result<String, serde_json::Error> {
    serde_json::to_string(values)
}

fn decode_string_list(value: &str) -> Result<Vec<String>, serde_json::Error> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(value)
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

fn u64_at(row: &turso::Row, index: usize) -> Result<u64, String> {
    let value: i64 = row.get(index).map_err(|err| err.to_string())?;
    u64::try_from(value).map_err(|err| err.to_string())
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

fn f32_at(row: &turso::Row, index: usize) -> Result<f32, String> {
    let value: f64 = row.get(index).map_err(|err| err.to_string())?;
    Ok(value as f32)
}

fn split_feature_set(feature_set: &str) -> Vec<&str> {
    feature_set
        .split(',')
        .filter(|feature| !feature.is_empty())
        .collect()
}
