CREATE TABLE IF NOT EXISTS snapshots (
    snapshot_id TEXT PRIMARY KEY,
    repo_root TEXT NOT NULL,
    worktree_root TEXT NOT NULL,
    git_sha TEXT NOT NULL,
    cargo_target TEXT NOT NULL,
    feature_set TEXT NOT NULL,
    cfg_profile TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS chunks (
    chunk_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    chunk_kind TEXT NOT NULL,
    symbol_path TEXT,
    symbol_name TEXT,
    owning_symbol_header TEXT,
    docs_text TEXT,
    signature_text TEXT,
    parent_symbol_path TEXT,
    retrieval_markers TEXT NOT NULL DEFAULT '',
    repository_state_hints TEXT NOT NULL DEFAULT '',
    file_path TEXT NOT NULL,
    start_byte INTEGER NOT NULL,
    end_byte INTEGER NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, chunk_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS edges (
    edge_id TEXT PRIMARY KEY,
    snapshot_id TEXT NOT NULL,
    from_chunk_id TEXT NOT NULL,
    to_chunk_id TEXT NOT NULL,
    edge_kind TEXT NOT NULL,
    from_symbol_path TEXT,
    to_symbol_path TEXT,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS indexing_runs (
    indexing_run_id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL,
    status TEXT NOT NULL,
    chunk_count INTEGER NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS query_audits (
    query_audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id TEXT NOT NULL,
    query_mode TEXT NOT NULL,
    query_text TEXT NOT NULL,
    result_count INTEGER NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS query_observations (
    observation_id TEXT PRIMARY KEY,
    snapshot_id TEXT NOT NULL,
    query_mode TEXT NOT NULL,
    query_text TEXT NOT NULL,
    symbol_path TEXT,
    changed_paths TEXT NOT NULL DEFAULT '',
    warnings TEXT NOT NULL DEFAULT '',
    result_count INTEGER NOT NULL,
    eval_task_id TEXT,
    evidence_class_coverage TEXT NOT NULL DEFAULT '',
    retrieval_config_json TEXT NOT NULL,
    observability_enabled INTEGER NOT NULL,
    observability_verbosity TEXT NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS candidate_observations (
    observation_id TEXT NOT NULL,
    rank INTEGER NOT NULL,
    chunk_id TEXT NOT NULL,
    chunk_kind TEXT NOT NULL,
    symbol_path TEXT,
    file_path TEXT NOT NULL,
    evidence TEXT NOT NULL DEFAULT '',
    retrieval_markers TEXT NOT NULL DEFAULT '',
    returned INTEGER NOT NULL,
    matched_worktree INTEGER NOT NULL,
    base_score REAL NOT NULL,
    query_mode_bias REAL NOT NULL,
    worktree_diff_bias REAL NOT NULL,
    final_score REAL NOT NULL,
    PRIMARY KEY (observation_id, rank, chunk_id),
    FOREIGN KEY (observation_id) REFERENCES query_observations(observation_id)
);

CREATE TABLE IF NOT EXISTS document_blocks (
    block_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    document_kind TEXT NOT NULL,
    parser TEXT NOT NULL,
    heading_path TEXT NOT NULL DEFAULT '',
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, block_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS history_nodes (
    node_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    node_kind TEXT NOT NULL,
    subject TEXT,
    summary TEXT NOT NULL,
    PRIMARY KEY (snapshot_id, node_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);

CREATE TABLE IF NOT EXISTS lineage_edges (
    edge_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    from_node_id TEXT NOT NULL,
    to_node_id TEXT NOT NULL,
    edge_kind TEXT NOT NULL,
    evidence TEXT,
    confidence REAL NOT NULL,
    PRIMARY KEY (snapshot_id, edge_id),
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
);
