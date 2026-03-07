use std::path::Path;

use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;
use tokio::runtime::Runtime;

use rarag_core::metadata::{IndexingRunRecord, QueryAuditRecord, SnapshotStore};

fn sample_snapshot(worktree_root: &str) -> SnapshotKey {
    SnapshotKey::new(
        "/repo",
        worktree_root,
        "abc123",
        "x86_64-unknown-linux-gnu",
        ["sqlite", "default", "sqlite"],
        "dev",
    )
}

fn with_runtime<T>(f: impl FnOnce(&Runtime) -> T) -> T {
    let runtime = Runtime::new().expect("tokio runtime");
    f(&runtime)
}

fn db_path(dir: &Path) -> String {
    dir.join("metadata.db").display().to_string()
}

#[test]
fn normalizes_feature_sets_before_insert() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");

            let snapshot = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/alpha"))
                .await
                .expect("insert snapshot");

            assert_eq!(snapshot.key.feature_set, vec!["default", "sqlite"]);
        })
    });
}

#[test]
fn same_build_world_reuses_snapshot_identity() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");

            let left = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/alpha"))
                .await
                .expect("insert left snapshot");
            let right = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/alpha"))
                .await
                .expect("insert right snapshot");

            assert_eq!(left.id, right.id);
        })
    });
}

#[test]
fn create_and_load_snapshot() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");

            let created = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/alpha"))
                .await
                .expect("create snapshot");
            store
                .record_indexing_run(IndexingRunRecord::new(created.id.clone(), "completed", 7))
                .await
                .expect("record indexing run");
            store
                .record_query_audit(QueryAuditRecord::new(
                    created.id.clone(),
                    "understand-symbol",
                    "query text",
                    3,
                ))
                .await
                .expect("record query audit");

            let loaded = store
                .load_snapshot(&created.id)
                .await
                .expect("load snapshot")
                .expect("snapshot exists");

            assert_eq!(loaded.id, created.id);
            assert_eq!(loaded.key.repo_root, "/repo");
            assert_eq!(loaded.last_indexed_chunk_count, Some(7));
            assert_eq!(loaded.last_query_mode.as_deref(), Some("understand-symbol"));
        })
    });
}
