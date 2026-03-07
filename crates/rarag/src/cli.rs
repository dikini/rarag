use std::path::PathBuf;

use rarag_core::daemon::{DaemonRequest, QueryPayload};
use rarag_core::retrieval::{QueryMode, WorkflowPhase};
use rarag_core::snapshot::SnapshotKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCommand {
    pub request: DaemonRequest,
    pub socket_path: PathBuf,
    pub json: bool,
    pub dry_run: bool,
}

pub fn parse_command(args: &[String], default_socket: &str, default_json: bool) -> Result<CliCommand, String> {
    let socket_path = option_value(args, "--socket")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_socket));
    let json = args.iter().any(|arg| arg == "--json") || default_json;
    let dry_run = args.iter().any(|arg| arg == "--dry-run-request");

    match args.get(1).map(String::as_str) {
        Some("status") => Ok(CliCommand {
            request: DaemonRequest::Status {
                snapshot_id: option_value(args, "--snapshot-id"),
                worktree_root: option_value(args, "--worktree-root"),
            },
            socket_path,
            json,
            dry_run,
        }),
        Some("index") => Ok(CliCommand {
            request: DaemonRequest::IndexWorkspace {
                snapshot: SnapshotKey::new(
                    required_option(args, "--repo-root")?,
                    required_option(args, "--worktree-root")?,
                    required_option(args, "--git-sha")?,
                    option_value(args, "--cargo-target")
                        .unwrap_or_else(|| "x86_64-unknown-linux-gnu".to_string()),
                    option_value(args, "--feature")
                        .unwrap_or_else(|| "default".to_string())
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .collect::<Vec<_>>(),
                    option_value(args, "--cfg-profile").unwrap_or_else(|| "dev".to_string()),
                ),
                workspace_root: required_option(args, "--workspace-root")?,
                max_body_bytes: option_value(args, "--max-body-bytes")
                    .as_deref()
                    .unwrap_or("80")
                    .parse()
                    .map_err(|err| format!("invalid --max-body-bytes: {err}"))?,
            },
            socket_path,
            json,
            dry_run,
        }),
        Some("query") => Ok(CliCommand {
            request: DaemonRequest::Query(parse_query_payload(args, false)?),
            socket_path,
            json,
            dry_run,
        }),
        Some("blast-radius") => Ok(CliCommand {
            request: DaemonRequest::BlastRadius(parse_query_payload(args, true)?),
            socket_path,
            json,
            dry_run,
        }),
        _ => Err("unsupported command".to_string()),
    }
}

fn parse_query_payload(args: &[String], force_blast_radius: bool) -> Result<QueryPayload, String> {
    Ok(QueryPayload {
        snapshot_id: option_value(args, "--snapshot-id"),
        worktree_root: option_value(args, "--worktree-root"),
        query_mode: if force_blast_radius {
            QueryMode::BlastRadius
        } else {
            parse_query_mode(&required_option(args, "--mode")?)?
        },
        workflow_phase: parse_workflow_phase(&required_option(args, "--phase")?)?,
        query_text: required_option(args, "--text")?,
        symbol_path: option_value(args, "--symbol-path"),
        limit: option_value(args, "--limit")
            .map(|value| value.parse().map_err(|err| format!("invalid --limit: {err}")))
            .transpose()?,
        changed_paths: repeated_values(args, "--changed-path"),
    })
}

fn parse_query_mode(value: &str) -> Result<QueryMode, String> {
    match value {
        "understand-symbol" => Ok(QueryMode::UnderstandSymbol),
        "implement-adjacent" => Ok(QueryMode::ImplementAdjacent),
        "bounded-refactor" => Ok(QueryMode::BoundedRefactor),
        "find-examples" => Ok(QueryMode::FindExamples),
        "blast-radius" => Ok(QueryMode::BlastRadius),
        _ => Err(format!("unsupported --mode {value}")),
    }
}

fn parse_workflow_phase(value: &str) -> Result<WorkflowPhase, String> {
    match value {
        "spec" => Ok(WorkflowPhase::Spec),
        "plan" => Ok(WorkflowPhase::Plan),
        "write-tests" => Ok(WorkflowPhase::WriteTests),
        "write-code" => Ok(WorkflowPhase::WriteCode),
        "verify" => Ok(WorkflowPhase::Verify),
        "review" => Ok(WorkflowPhase::Review),
        "fix" => Ok(WorkflowPhase::Fix),
        _ => Err(format!("unsupported --phase {value}")),
    }
}

fn required_option(args: &[String], flag: &str) -> Result<String, String> {
    option_value(args, flag).ok_or_else(|| format!("missing required flag {flag}"))
}

fn option_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}

fn repeated_values(args: &[String], flag: &str) -> Vec<String> {
    args.windows(2)
        .filter(|window| window[0] == flag)
        .map(|window| window[1].clone())
        .collect()
}
