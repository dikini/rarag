use std::path::Path;

use rarag_core::snapshot::SnapshotKey;
use tempfile::tempdir;
use tokio::runtime::Runtime;

use rarag_core::config::{ObservabilityConfig, ObservabilityVerbosity, RetrievalConfig};
use rarag_core::metadata::{
    CandidateObservationRecord, DocumentBlockRecord, HistoryNodeRecord, IndexingRunRecord,
    LineageEdgeRecord, QueryAuditRecord, QueryObservationRecord, SnapshotStore,
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
                vec![
                    "src/lib.rs".to_string(),
                    "src/path,with,comma.rs".to_string(),
                ],
                vec![
                    "warning".to_string(),
                    "provider said: keep,trim,rank".to_string(),
                ],
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
                vec!["exact_symbol".to_string(), "semantic,seed".to_string()],
                vec!["example".to_string(), "tests,docs".to_string()],
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
            assert_eq!(loaded[0].changed_paths, observation.changed_paths);
            assert_eq!(loaded[0].warnings, observation.warnings);
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

#[test]
fn record_query_observation_is_atomic_for_candidate_failures() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");
            let snapshot = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/obs-atomic"))
                .await
                .expect("create snapshot");

            let observation = QueryObservationRecord::new(
                "obs-atomic",
                snapshot.id.clone(),
                "understand-symbol",
                "atomic observation write",
                None,
                vec!["src/lib.rs".to_string()],
                Vec::new(),
                2,
                RetrievalConfig::default(),
                ObservabilityConfig {
                    enabled: true,
                    verbosity: ObservabilityVerbosity::Summary,
                },
            );
            let candidates = vec![
                CandidateObservationRecord::new(
                    "obs-atomic",
                    "chunk-dup",
                    "Symbol",
                    None,
                    "src/lib.rs",
                    vec!["exact_symbol".to_string()],
                    vec!["symbol".to_string()],
                    1,
                    true,
                    true,
                    5.0,
                    0.8,
                    0.0,
                    5.8,
                ),
                CandidateObservationRecord::new(
                    "obs-atomic",
                    "chunk-dup",
                    "Symbol",
                    None,
                    "src/lib.rs",
                    vec!["exact_symbol".to_string()],
                    vec!["symbol".to_string()],
                    1,
                    false,
                    true,
                    4.0,
                    0.8,
                    0.0,
                    4.8,
                ),
            ];

            let error = store
                .record_query_observation(observation, &candidates)
                .await
                .expect_err("duplicate candidate rows should fail");
            assert!(
                error.contains("UNIQUE") || error.contains("constraint"),
                "unexpected error: {error}"
            );

            let observations = store
                .load_query_observations(&snapshot.id)
                .await
                .expect("load observations after failed write");
            assert!(observations.is_empty(), "observation row leaked on failure");

            let loaded_candidates = store
                .load_candidate_observations("obs-atomic")
                .await
                .expect("load candidates after failed write");
            assert!(
                loaded_candidates.is_empty(),
                "candidate rows leaked on failure"
            );
        })
    });
}

#[test]
fn document_and_history_rows_roundtrip_without_cross_snapshot_leakage() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");
            let alpha = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/doc-alpha"))
                .await
                .expect("create alpha snapshot");
            let beta = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/doc-beta"))
                .await
                .expect("create beta snapshot");

            store
                .replace_document_blocks(
                    &alpha.id,
                    &[DocumentBlockRecord::new(
                        "doc-1",
                        alpha.id.clone(),
                        "docs/specs/repository-rag-architecture.md",
                        "spec",
                        "markdown",
                        vec!["Repository RAG".to_string()],
                        1,
                        12,
                        "Canonical behavior.",
                    )],
                )
                .await
                .expect("write alpha document block");
            store
                .replace_document_blocks(
                    &beta.id,
                    &[DocumentBlockRecord::new(
                        "doc-2",
                        beta.id.clone(),
                        "docs/plans/future.md",
                        "plan",
                        "markdown",
                        vec!["Future".to_string()],
                        1,
                        9,
                        "Future work.",
                    )],
                )
                .await
                .expect("write beta document block");

            store
                .replace_history_nodes(
                    &alpha.id,
                    &[HistoryNodeRecord::new(
                        "hist-1",
                        alpha.id.clone(),
                        "commit",
                        Some("abc123".to_string()),
                        "Introduced retrieval observation pipeline",
                    )],
                )
                .await
                .expect("write alpha history");
            store
                .replace_lineage_edges(
                    &alpha.id,
                    &[LineageEdgeRecord::new(
                        "line-1",
                        alpha.id.clone(),
                        "hist-1",
                        "doc-1",
                        "introduced_invariant",
                        Some("CHANGELOG entry".to_string()),
                        0.75,
                    )],
                )
                .await
                .expect("write alpha lineage");

            let alpha_docs = store
                .load_document_blocks(&alpha.id)
                .await
                .expect("load alpha docs");
            let beta_docs = store
                .load_document_blocks(&beta.id)
                .await
                .expect("load beta docs");
            let alpha_history = store
                .load_history_nodes(&alpha.id)
                .await
                .expect("load alpha history");
            let beta_history = store
                .load_history_nodes(&beta.id)
                .await
                .expect("load beta history");
            let alpha_edges = store
                .load_lineage_edges(&alpha.id)
                .await
                .expect("load alpha edges");
            let beta_edges = store
                .load_lineage_edges(&beta.id)
                .await
                .expect("load beta edges");

            assert_eq!(alpha_docs.len(), 1);
            assert_eq!(beta_docs.len(), 1);
            assert_eq!(alpha_docs[0].block_id, "doc-1");
            assert_eq!(beta_docs[0].block_id, "doc-2");
            assert_eq!(alpha_history.len(), 1);
            assert!(beta_history.is_empty());
            assert_eq!(alpha_edges.len(), 1);
            assert!(beta_edges.is_empty());
        })
    });
}

#[test]
fn stores_document_blocks_history_nodes_and_observations() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");
            let snapshot = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/doc-history"))
                .await
                .expect("create snapshot");

            store
                .replace_document_blocks(
                    &snapshot.id,
                    &[DocumentBlockRecord::new(
                        "doc-obs",
                        snapshot.id.clone(),
                        "docs/ops/systemd-user.md",
                        "ops",
                        "markdown",
                        vec!["Reload".to_string()],
                        10,
                        33,
                        "Use systemctl --user reload or SIGHUP.",
                    )],
                )
                .await
                .expect("write document");
            store
                .replace_history_nodes(
                    &snapshot.id,
                    &[HistoryNodeRecord::new(
                        "hist-obs",
                        snapshot.id.clone(),
                        "change",
                        Some("def456".to_string()),
                        "Added daemon config reload",
                    )],
                )
                .await
                .expect("write history");

            let observation = QueryObservationRecord::new(
                "obs-doc-history",
                snapshot.id.clone(),
                "blast-radius",
                "how was daemon reload introduced",
                Some("rarag::daemon::reload".to_string()),
                vec!["crates/raragd/src/server.rs".to_string()],
                Vec::new(),
                1,
                RetrievalConfig::default(),
                ObservabilityConfig {
                    enabled: true,
                    verbosity: ObservabilityVerbosity::Summary,
                },
            )
            .with_eval(
                Some("reload-archaeology".to_string()),
                vec![
                    "code".to_string(),
                    "document".to_string(),
                    "history".to_string(),
                ],
            );
            let candidates = vec![CandidateObservationRecord::new(
                "obs-doc-history",
                "doc-obs",
                "DocumentBlock",
                None,
                "docs/ops/systemd-user.md",
                vec!["lexical_bm25".to_string()],
                vec!["ops".to_string()],
                1,
                true,
                false,
                4.0,
                0.2,
                0.0,
                4.2,
            )];

            store
                .record_query_observation(observation, &candidates)
                .await
                .expect("record observation");

            let loaded_docs = store
                .load_document_blocks(&snapshot.id)
                .await
                .expect("load docs");
            let loaded_history = store
                .load_history_nodes(&snapshot.id)
                .await
                .expect("load history");
            let loaded_obs = store
                .load_query_observations(&snapshot.id)
                .await
                .expect("load observations");

            assert_eq!(loaded_docs.len(), 1);
            assert_eq!(loaded_history.len(), 1);
            assert_eq!(loaded_obs.len(), 1);
            assert_eq!(
                loaded_obs[0].eval_task_id.as_deref(),
                Some("reload-archaeology")
            );
            assert_eq!(
                loaded_obs[0].evidence_class_coverage,
                vec![
                    "code".to_string(),
                    "document".to_string(),
                    "history".to_string()
                ]
            );
        })
    });
}

#[test]
fn stores_history_nodes_and_lineage_edges() {
    with_runtime(|runtime| {
        runtime.block_on(async {
            let dir = tempdir().expect("tempdir");
            let store = SnapshotStore::open_local(&db_path(dir.path()))
                .await
                .expect("open local db");
            let snapshot = store
                .create_or_get_snapshot(sample_snapshot("/repo/.worktrees/history-store"))
                .await
                .expect("create snapshot");

            store
                .replace_history_nodes(
                    &snapshot.id,
                    &[HistoryNodeRecord::new(
                        "hist-1",
                        snapshot.id.clone(),
                        "commit",
                        Some("abc123".to_string()),
                        "Introduced reload behavior",
                    )],
                )
                .await
                .expect("write history");
            store
                .replace_lineage_edges(
                    &snapshot.id,
                    &[LineageEdgeRecord::new(
                        "line-1",
                        snapshot.id.clone(),
                        "hist-1",
                        "hist-1",
                        "followed_by",
                        Some("same commit lineage anchor".to_string()),
                        1.0,
                    )],
                )
                .await
                .expect("write lineage");

            let history = store
                .load_history_nodes(&snapshot.id)
                .await
                .expect("load history");
            let edges = store
                .load_lineage_edges(&snapshot.id)
                .await
                .expect("load lineage");

            assert_eq!(history.len(), 1);
            assert_eq!(edges.len(), 1);
            assert_eq!(history[0].node_id, "hist-1");
            assert_eq!(edges[0].edge_id, "line-1");
        })
    });
}
