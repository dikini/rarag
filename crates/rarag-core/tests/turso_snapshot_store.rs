use std::path::Path;

use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;
use tokio::runtime::Runtime;

use rarag_core::config::{ObservabilityConfig, ObservabilityVerbosity, RetrievalConfig};
use rarag_core::metadata::{
    CandidateObservationRecord, IndexingRunRecord, QueryAuditRecord, QueryObservationRecord,
    SnapshotStore,
};

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

#[test]
fn resolves_latest_snapshot_for_worktree_root() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");

            let first = store
                .create_or_get_snapshot(SnapshotKey::new(
                    "/repo",
                    "/repo/.worktrees/alpha",
                    "abc123",
                    "x86_64-unknown-linux-gnu",
                    ["default"],
                    "dev",
                ))
                .await
                .expect("create first snapshot");
            let second = store
                .create_or_get_snapshot(SnapshotKey::new(
                    "/repo",
                    "/repo/.worktrees/alpha",
                    "def456",
                    "x86_64-unknown-linux-gnu",
                    ["default"],
                    "dev",
                ))
                .await
                .expect("create second snapshot");

            let resolved = store
                .resolve_snapshot_for_worktree_root("/repo/.worktrees/alpha")
                .await
                .expect("resolve snapshot")
                .expect("snapshot exists");

            assert_eq!(resolved.id, second.id);
            assert_ne!(resolved.id, first.id);
            assert_eq!(resolved.key.git_sha, "def456");
        })
    });
}

#[test]
fn records_and_loads_query_observations() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");
            let snapshot = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/obs"))
                .await
                .expect("create snapshot");

            let observation = QueryObservationRecord::new(
                "obs-1",
                snapshot.id.clone(),
                "find-examples",
                "example_sum",
                Some("mini_repo::example_sum".to_string()),
                vec!["src/lib.rs".to_string()],
                vec!["warning".to_string()],
                2,
                RetrievalConfig::default(),
                ObservabilityConfig {
                    enabled: true,
                    verbosity: ObservabilityVerbosity::Detailed,
                },
            );
            let candidates = vec![CandidateObservationRecord::new(
                "obs-1",
                "chunk-1",
                "Symbol",
                Some("mini_repo::example_sum".to_string()),
                "src/lib.rs",
                vec!["exact_symbol".to_string()],
                vec!["example".to_string()],
                1,
                true,
                false,
                10.0,
                0.8,
                0.0,
                10.8,
            )];

            store
                .record_query_observation(observation.clone(), &candidates)
                .await
                .expect("record query observation");

            let loaded = store
                .load_query_observations(&snapshot.id)
                .await
                .expect("load observations");
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].observation_id, "obs-1");
            assert_eq!(loaded[0].changed_paths, vec!["src/lib.rs"]);
            assert_eq!(
                loaded[0].observability.verbosity,
                ObservabilityVerbosity::Detailed
            );

            let loaded_candidates = store
                .load_candidate_observations("obs-1")
                .await
                .expect("load candidate observations");
            assert_eq!(loaded_candidates, candidates);
        })
    });
}
