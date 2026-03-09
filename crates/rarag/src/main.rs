mod cli;
mod client;
mod services;

use serde::Serialize;

use rarag_core::daemon::{DaemonRequest, DaemonResponse, QueryPayload};
use rarag_core::retrieval::{EvalTaskFixture, RetrievedChunk, load_eval_task_fixtures};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let config_path = args
        .windows(2)
        .find(|window| window[0] == "--config")
        .map(|window| std::path::PathBuf::from(&window[1]));
    let rarag_core::config_loader::LoadedAppConfig {
        config,
        source_path,
    } = rarag_core::config_loader::load_app_config_with_source(config_path.as_deref())
        .expect("load app config");

    if args.iter().any(|arg| arg == "--print-config") {
        println!("binary=rarag");
        println!("default_json={}", config.cli_default_json());
        println!("embedding_model={}", config.embeddings.model);
        return;
    }

    match cli::parse_command(
        &args,
        config.daemon_socket_path(),
        config.cli_default_json(),
    ) {
        Ok(command) => match &command.action {
            cli::CliAction::DaemonRequest(request) => {
                if command.dry_run {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(request).expect("serialize request")
                    );
                    return;
                }

                match client::send_request(&command.socket_path, request) {
                    Ok(response) => {
                        if command.json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&response)
                                    .expect("serialize response")
                            );
                        } else {
                            print_human(&response);
                        }
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        std::process::exit(1);
                    }
                }
            }
            cli::CliAction::Service(service_command) => {
                let service_context =
                    services::ServiceInstallContext::discover(source_path.clone())
                        .expect("resolve service install paths");
                let result = if command.dry_run {
                    services::plan(service_command, &service_context)
                } else {
                    services::execute(service_command, &service_context)
                };
                match result {
                    Ok(report) => {
                        if command.json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&report)
                                    .expect("serialize service report")
                            );
                        } else {
                            services::print_human(&report);
                        }
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        std::process::exit(1);
                    }
                }
            }
            cli::CliAction::EvalReplay(eval_command) => {
                if command.dry_run {
                    let fixtures = load_eval_task_fixtures(&eval_command.fixtures_path)
                        .expect("load eval fixtures");
                    let dry_run_requests: Vec<_> = fixtures
                        .iter()
                        .map(|task| DaemonRequest::Query(eval_task_to_payload(task, eval_command)))
                        .collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&dry_run_requests)
                            .expect("serialize eval requests")
                    );
                    return;
                }
                match run_eval_replay(eval_command, &command.socket_path) {
                    Ok(report) => {
                        if command.json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&report)
                                    .expect("serialize eval replay report")
                            );
                        } else {
                            print_eval_report_human(&report);
                        }
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        std::process::exit(1);
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("{err}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!("rarag cli");
    println!("Usage:");
    println!("  rarag [--help] [--config <path>] [--print-config]");
    println!("  rarag service install [--force] [--json] [--dry-run-request]");
    println!(
        "  rarag service start [--service <raragd|rarag-mcp|all>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag service stop [--service <raragd|rarag-mcp|all>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag service restart [--service <raragd|rarag-mcp|all>] [--json] [--dry-run-request]"
    );
    println!("  rarag service reload [--service <raragd|all>] [--json] [--dry-run-request]");
    println!(
        "  rarag status [--socket <path>] [--snapshot-id <id> | --worktree-root <path>] [--json]"
    );
    println!("  rarag daemon reload [--socket <path>] [--json] [--dry-run-request]");
    println!(
        "  rarag index build --workspace-root <path> --repo-root <path> --worktree <path> --git-sha <sha> [--cargo-target <triple>] [--feature <csv>] [--cfg-profile <profile>] [--max-body-bytes <n>] [--socket <path>] [--json]"
    );
    println!(
        "  rarag index status [--socket <path>] [--snapshot <id> | --worktree <path>] [--json]"
    );
    println!(
        "  rarag query --mode <mode> --text <query> [--symbol-path <path>] [--snapshot <id> | --worktree <path>] [--changed-path <path>]... [--include-history] [--history-max-nodes <n>] [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag symbol --text <query> [--symbol-path <path>] [--snapshot <id> | --worktree <path>] [--changed-path <path>]... [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag examples --text <query> [--symbol-path <path>] [--snapshot <id> | --worktree <path>] [--changed-path <path>]... [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag blast-radius --text <query> [--symbol-path <path>] [--snapshot <id> | --worktree <path>] [--changed-path <path>]... [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag eval replay --fixtures <path> [--snapshot <id> | --worktree <path>] [--include-history] [--history-max-nodes <n>] [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
    println!("  rarag doctor [--socket <path>] [--json] [--dry-run-request]");
}

#[derive(Debug, Clone, Serialize)]
struct EvalReplayTaskReport {
    task_id: String,
    query_mode: String,
    query_text: String,
    result_count: usize,
    ideal_hits: usize,
    acceptable_hits: usize,
    distractor_hits: usize,
    matched_ideal: Vec<String>,
    matched_acceptable: Vec<String>,
    matched_distractors: Vec<String>,
    evidence_class_coverage: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EvalReplayReport {
    fixtures_path: String,
    task_count: usize,
    ideal_task_hit_rate: f32,
    acceptable_task_hit_rate: f32,
    distractor_task_hit_rate: f32,
    tasks: Vec<EvalReplayTaskReport>,
}

fn run_eval_replay(
    command: &cli::EvalReplayCommand,
    socket_path: &std::path::Path,
) -> Result<EvalReplayReport, String> {
    let tasks = load_eval_task_fixtures(&command.fixtures_path)?;
    if tasks.is_empty() {
        return Err(format!(
            "no eval tasks loaded from {}",
            command.fixtures_path.display()
        ));
    }
    let mut reports = Vec::with_capacity(tasks.len());
    for task in &tasks {
        let request = DaemonRequest::Query(eval_task_to_payload(task, command));
        let response = client::send_request(socket_path, &request)?;
        let DaemonResponse::Query(payload) = response else {
            return Err("unexpected daemon response for eval replay task".to_string());
        };

        let matched_ideal = matched_patterns(&payload.items, &task.ideal);
        let matched_acceptable = matched_patterns(&payload.items, &task.acceptable);
        let matched_distractors = matched_patterns(&payload.items, &task.distractors);
        reports.push(EvalReplayTaskReport {
            task_id: task.task_id.clone(),
            query_mode: task.query_mode.clone(),
            query_text: task.query_text.clone(),
            result_count: payload.items.len(),
            ideal_hits: matched_ideal.len(),
            acceptable_hits: matched_acceptable.len(),
            distractor_hits: matched_distractors.len(),
            matched_ideal,
            matched_acceptable,
            matched_distractors,
            evidence_class_coverage: evidence_class_coverage(&payload.items),
            warnings: payload.warnings,
        });
    }

    let task_count = reports.len();
    let ideal_task_hits = reports.iter().filter(|task| task.ideal_hits > 0).count();
    let acceptable_task_hits = reports
        .iter()
        .filter(|task| task.ideal_hits > 0 || task.acceptable_hits > 0)
        .count();
    let distractor_task_hits = reports
        .iter()
        .filter(|task| task.distractor_hits > 0)
        .count();

    Ok(EvalReplayReport {
        fixtures_path: command.fixtures_path.display().to_string(),
        task_count,
        ideal_task_hit_rate: ideal_task_hits as f32 / task_count as f32,
        acceptable_task_hit_rate: acceptable_task_hits as f32 / task_count as f32,
        distractor_task_hit_rate: distractor_task_hits as f32 / task_count as f32,
        tasks: reports,
    })
}

fn eval_task_to_payload(task: &EvalTaskFixture, command: &cli::EvalReplayCommand) -> QueryPayload {
    let mut include_history = command.include_history;
    if !include_history {
        include_history = task
            .ideal
            .iter()
            .chain(task.acceptable.iter())
            .any(|item| item.starts_with("history:"));
    }
    QueryPayload {
        snapshot_id: command.snapshot_id.clone(),
        worktree_root: command.worktree_root.clone(),
        query_mode: cli::parse_query_mode(&task.query_mode).expect("valid fixture query mode"),
        query_text: task.query_text.clone(),
        symbol_path: task.symbol_path.clone(),
        limit: command.limit,
        changed_paths: Vec::new(),
        include_history,
        history_max_nodes: command.history_max_nodes,
        eval_task_id: Some(task.task_id.clone()),
    }
}

fn matched_patterns(items: &[RetrievedChunk], patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .filter(|pattern| item_matches_any_pattern(items, pattern))
        .cloned()
        .collect()
}

fn item_matches_any_pattern(items: &[RetrievedChunk], pattern: &str) -> bool {
    items.iter().any(|item| {
        item.chunk.chunk_id.contains(pattern)
            || item.chunk.file_path.contains(pattern)
            || item
                .chunk
                .symbol_path
                .as_deref()
                .is_some_and(|symbol_path| symbol_path.contains(pattern))
            || item.chunk.text.contains(pattern)
    })
}

fn evidence_class_coverage(items: &[RetrievedChunk]) -> Vec<String> {
    let mut coverage = Vec::new();
    if items
        .iter()
        .any(|item| item.chunk.chunk_kind == "HistoryNode")
    {
        coverage.push("history".to_string());
    }
    if items.iter().any(|item| {
        item.chunk.chunk_kind == "DocumentBlock"
            || item.chunk.chunk_kind == "TaskRow"
            || item
                .chunk
                .retrieval_markers
                .iter()
                .any(|marker| marker == "document")
    }) {
        coverage.push("document".to_string());
    }
    if items.iter().any(|item| {
        !matches!(
            item.chunk.chunk_kind.as_str(),
            "HistoryNode" | "DocumentBlock" | "TaskRow"
        )
    }) {
        coverage.push("code".to_string());
    }
    coverage
}

fn print_eval_report_human(report: &EvalReplayReport) {
    println!("fixtures={}", report.fixtures_path);
    println!("tasks={}", report.task_count);
    println!("ideal_task_hit_rate={:.3}", report.ideal_task_hit_rate);
    println!(
        "acceptable_task_hit_rate={:.3}",
        report.acceptable_task_hit_rate
    );
    println!(
        "distractor_task_hit_rate={:.3}",
        report.distractor_task_hit_rate
    );
    for task in &report.tasks {
        println!("task={}", task.task_id);
        println!(
            "  mode={} ideal_hits={} acceptable_hits={} distractor_hits={} result_count={}",
            task.query_mode,
            task.ideal_hits,
            task.acceptable_hits,
            task.distractor_hits,
            task.result_count
        );
    }
}

fn print_human(response: &rarag_core::daemon::DaemonResponse) {
    match response {
        rarag_core::daemon::DaemonResponse::Status(payload) => {
            println!(
                "resolved_snapshot_id={}",
                payload.resolved_snapshot_id.as_deref().unwrap_or_default()
            );
        }
        rarag_core::daemon::DaemonResponse::Indexed(payload) => {
            println!("snapshot_id={}", payload.snapshot_id);
            println!("chunk_count={}", payload.chunk_count);
        }
        rarag_core::daemon::DaemonResponse::Query(payload) => {
            println!("items={}", payload.items.len());
            if let Some(top) = payload.items.first() {
                println!(
                    "top_symbol={}",
                    top.chunk.symbol_path.as_deref().unwrap_or_default()
                );
            }
        }
        rarag_core::daemon::DaemonResponse::Reloaded(payload) => {
            println!("generation={}", payload.generation);
            println!(
                "source_path={}",
                payload.source_path.as_deref().unwrap_or_default()
            );
        }
        rarag_core::daemon::DaemonResponse::Ack => println!("ack"),
        rarag_core::daemon::DaemonResponse::Error(err) => {
            println!("error={}", err.message);
        }
    }
}
