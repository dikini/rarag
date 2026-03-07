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
