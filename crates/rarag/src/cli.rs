use std::path::PathBuf;

use rarag_core::daemon::{DaemonRequest, QueryPayload};
use rarag_core::retrieval::QueryMode;
use rarag_core::snapshot::SnapshotKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliCommand {
    pub action: CliAction,
    pub socket_path: PathBuf,
    pub json: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    DaemonRequest(DaemonRequest),
    Service(ServiceCommand),
    EvalReplay(EvalReplayCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCommand {
    pub operation: ServiceOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalReplayCommand {
    pub fixtures_path: PathBuf,
    pub snapshot_id: Option<String>,
    pub worktree_root: Option<String>,
    pub include_history: bool,
    pub history_max_nodes: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceOperation {
    Install { force: bool },
    Start { target: ServiceTarget },
    Stop { target: ServiceTarget },
    Restart { target: ServiceTarget },
    Reload { target: ServiceTarget },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceTarget {
    All,
    Daemon,
    Mcp,
}

pub fn parse_command(
    args: &[String],
    default_socket: &str,
    default_json: bool,
) -> Result<CliCommand, String> {
    let socket_path = option_value(args, "--socket")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_socket));
    let json = args.iter().any(|arg| arg == "--json") || default_json;
    let dry_run = args.iter().any(|arg| arg == "--dry-run-request");
    let primary = args.get(1).map(String::as_str);
    let secondary = args.get(2).map(String::as_str);

    match (primary, secondary) {
        (Some("daemon"), Some("reload")) => Ok(CliCommand {
            action: CliAction::DaemonRequest(DaemonRequest::ReloadConfig),
            socket_path,
            json,
            dry_run,
        }),
        (Some("service"), Some(subcommand)) => Ok(CliCommand {
            action: CliAction::Service(ServiceCommand {
                operation: parse_service_operation(subcommand, args)?,
            }),
            socket_path,
            json,
            dry_run,
        }),
        (Some("eval"), Some("replay")) => Ok(CliCommand {
            action: CliAction::EvalReplay(parse_eval_replay_command(args)?),
            socket_path,
            json,
            dry_run,
        }),
        (Some("status"), _) | (Some("index"), Some("status")) | (Some("doctor"), _) => {
            Ok(CliCommand {
                action: CliAction::DaemonRequest(DaemonRequest::Status {
                    snapshot_id: locator_option(args, "--snapshot-id", "--snapshot"),
                    worktree_root: locator_option(args, "--worktree-root", "--worktree"),
                }),
                socket_path,
                json,
                dry_run,
            })
        }
        (Some("index"), None) | (Some("index"), Some("build")) | (Some("index"), Some(_))
            if index_build_compat_mode(secondary) =>
        {
            Ok(CliCommand {
                action: CliAction::DaemonRequest(DaemonRequest::IndexWorkspace {
                    snapshot: SnapshotKey::new(
                        required_option(args, "--repo-root")?,
                        required_option_alias(args, "--worktree-root", "--worktree")?,
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
                }),
                socket_path,
                json,
                dry_run,
            })
        }
        (Some("query"), _) => Ok(CliCommand {
            action: CliAction::DaemonRequest(DaemonRequest::Query(parse_query_payload(
                args, false,
            )?)),
            socket_path,
            json,
            dry_run,
        }),
        (Some("symbol"), _) => Ok(CliCommand {
            action: CliAction::DaemonRequest(DaemonRequest::Query(parse_named_query_payload(
                args,
                QueryMode::UnderstandSymbol,
            )?)),
            socket_path,
            json,
            dry_run,
        }),
        (Some("examples"), _) => Ok(CliCommand {
            action: CliAction::DaemonRequest(DaemonRequest::Query(parse_named_query_payload(
                args,
                QueryMode::FindExamples,
            )?)),
            socket_path,
            json,
            dry_run,
        }),
        (Some("blast-radius"), _) => Ok(CliCommand {
            action: CliAction::DaemonRequest(DaemonRequest::BlastRadius(parse_query_payload(
                args, true,
            )?)),
            socket_path,
            json,
            dry_run,
        }),
        _ => Err("unsupported command".to_string()),
    }
}

fn parse_query_payload(args: &[String], force_blast_radius: bool) -> Result<QueryPayload, String> {
    Ok(QueryPayload {
        snapshot_id: locator_option(args, "--snapshot-id", "--snapshot"),
        worktree_root: locator_option(args, "--worktree-root", "--worktree"),
        query_mode: if force_blast_radius {
            QueryMode::BlastRadius
        } else {
            parse_query_mode(&required_option(args, "--mode")?)?
        },
        query_text: required_option(args, "--text")?,
        symbol_path: option_value(args, "--symbol-path"),
        limit: option_value(args, "--limit")
            .map(|value| {
                value
                    .parse()
                    .map_err(|err| format!("invalid --limit: {err}"))
            })
            .transpose()?,
        changed_paths: repeated_values(args, "--changed-path"),
        include_history: args.iter().any(|arg| arg == "--include-history"),
        history_max_nodes: option_value(args, "--history-max-nodes")
            .map(|value| {
                value
                    .parse()
                    .map_err(|err| format!("invalid --history-max-nodes: {err}"))
            })
            .transpose()?,
        eval_task_id: option_value(args, "--eval-task-id"),
    })
}

fn parse_named_query_payload(
    args: &[String],
    query_mode: QueryMode,
) -> Result<QueryPayload, String> {
    Ok(QueryPayload {
        snapshot_id: locator_option(args, "--snapshot-id", "--snapshot"),
        worktree_root: locator_option(args, "--worktree-root", "--worktree"),
        query_mode,
        query_text: required_option(args, "--text")?,
        symbol_path: option_value(args, "--symbol-path"),
        limit: option_value(args, "--limit")
            .map(|value| {
                value
                    .parse()
                    .map_err(|err| format!("invalid --limit: {err}"))
            })
            .transpose()?,
        changed_paths: repeated_values(args, "--changed-path"),
        include_history: args.iter().any(|arg| arg == "--include-history"),
        history_max_nodes: option_value(args, "--history-max-nodes")
            .map(|value| {
                value
                    .parse()
                    .map_err(|err| format!("invalid --history-max-nodes: {err}"))
            })
            .transpose()?,
        eval_task_id: option_value(args, "--eval-task-id"),
    })
}

pub fn parse_query_mode(value: &str) -> Result<QueryMode, String> {
    match value {
        "understand-symbol" => Ok(QueryMode::UnderstandSymbol),
        "implement-adjacent" => Ok(QueryMode::ImplementAdjacent),
        "bounded-refactor" => Ok(QueryMode::BoundedRefactor),
        "find-examples" => Ok(QueryMode::FindExamples),
        "blast-radius" => Ok(QueryMode::BlastRadius),
        _ => Err(format!("unsupported --mode {value}")),
    }
}

fn parse_eval_replay_command(args: &[String]) -> Result<EvalReplayCommand, String> {
    let snapshot_id = locator_option(args, "--snapshot-id", "--snapshot");
    let worktree_root = locator_option(args, "--worktree-root", "--worktree");
    if snapshot_id.is_none() && worktree_root.is_none() {
        return Err("eval replay requires --snapshot or --worktree".to_string());
    }
    Ok(EvalReplayCommand {
        fixtures_path: PathBuf::from(required_option(args, "--fixtures")?),
        snapshot_id,
        worktree_root,
        include_history: args.iter().any(|arg| arg == "--include-history"),
        history_max_nodes: option_value(args, "--history-max-nodes")
            .map(|value| {
                value
                    .parse()
                    .map_err(|err| format!("invalid --history-max-nodes: {err}"))
            })
            .transpose()?,
        limit: option_value(args, "--limit")
            .map(|value| {
                value
                    .parse()
                    .map_err(|err| format!("invalid --limit: {err}"))
            })
            .transpose()?,
    })
}

fn required_option(args: &[String], flag: &str) -> Result<String, String> {
    option_value(args, flag).ok_or_else(|| format!("missing required flag {flag}"))
}

fn required_option_alias(
    args: &[String],
    primary_flag: &str,
    alias_flag: &str,
) -> Result<String, String> {
    locator_option(args, primary_flag, alias_flag)
        .ok_or_else(|| format!("missing required flag {primary_flag}"))
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

fn locator_option(args: &[String], primary_flag: &str, alias_flag: &str) -> Option<String> {
    option_value(args, primary_flag).or_else(|| option_value(args, alias_flag))
}

fn index_build_compat_mode(secondary: Option<&str>) -> bool {
    secondary.is_none()
        || secondary == Some("build")
        || secondary.is_some_and(|flag| flag.starts_with('-'))
}

fn parse_service_operation(subcommand: &str, args: &[String]) -> Result<ServiceOperation, String> {
    match subcommand {
        "install" => Ok(ServiceOperation::Install {
            force: args.iter().any(|arg| arg == "--force"),
        }),
        "start" => Ok(ServiceOperation::Start {
            target: parse_service_target(args)?,
        }),
        "stop" => Ok(ServiceOperation::Stop {
            target: parse_service_target(args)?,
        }),
        "restart" => Ok(ServiceOperation::Restart {
            target: parse_service_target(args)?,
        }),
        "reload" => {
            let target = parse_service_target(args)?;
            if target == ServiceTarget::Mcp {
                return Err("reload only supports raragd or all".to_string());
            }
            Ok(ServiceOperation::Reload { target })
        }
        _ => Err(format!("unsupported service command {subcommand}")),
    }
}

fn parse_service_target(args: &[String]) -> Result<ServiceTarget, String> {
    match option_value(args, "--service").as_deref() {
        None | Some("all") => Ok(ServiceTarget::All),
        Some("raragd") => Ok(ServiceTarget::Daemon),
        Some("rarag-mcp") => Ok(ServiceTarget::Mcp),
        Some(other) => Err(format!("unsupported --service value {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_service_install_force() {
        let parsed = parse_command(
            &args(&["rarag", "service", "install", "--force"]),
            "/tmp/raragd.sock",
            false,
        )
        .expect("parse command");
        assert_eq!(
            parsed.action,
            CliAction::Service(ServiceCommand {
                operation: ServiceOperation::Install { force: true },
            })
        );
    }

    #[test]
    fn parses_service_start_default_target_all() {
        let parsed = parse_command(
            &args(&["rarag", "service", "start"]),
            "/tmp/raragd.sock",
            false,
        )
        .expect("parse command");
        assert_eq!(
            parsed.action,
            CliAction::Service(ServiceCommand {
                operation: ServiceOperation::Start {
                    target: ServiceTarget::All
                },
            })
        );
    }

    #[test]
    fn rejects_reload_for_mcp_target() {
        let err = parse_command(
            &args(&["rarag", "service", "reload", "--service", "rarag-mcp"]),
            "/tmp/raragd.sock",
            false,
        )
        .expect_err("parse error");
        assert!(err.contains("reload only supports raragd or all"));
    }

    #[test]
    fn parses_eval_replay_with_worktree() {
        let parsed = parse_command(
            &args(&[
                "rarag",
                "eval",
                "replay",
                "--fixtures",
                "tests/fixtures/eval/tasks.json",
                "--worktree",
                "/repo",
                "--include-history",
                "--history-max-nodes",
                "4",
                "--limit",
                "8",
            ]),
            "/tmp/raragd.sock",
            false,
        )
        .expect("parse command");
        assert_eq!(
            parsed.action,
            CliAction::EvalReplay(EvalReplayCommand {
                fixtures_path: PathBuf::from("tests/fixtures/eval/tasks.json"),
                snapshot_id: None,
                worktree_root: Some("/repo".to_string()),
                include_history: true,
                history_max_nodes: Some(4),
                limit: Some(8),
            })
        );
    }

    #[test]
    fn rejects_eval_replay_without_locator() {
        let err = parse_command(
            &args(&[
                "rarag",
                "eval",
                "replay",
                "--fixtures",
                "tests/fixtures/eval/tasks.json",
            ]),
            "/tmp/raragd.sock",
            false,
        )
        .expect_err("parse error");
        assert_eq!(err, "eval replay requires --snapshot or --worktree");
    }
}
