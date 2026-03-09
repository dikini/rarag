mod cli;
mod client;
mod services;

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
                let service_context = services::ServiceInstallContext::discover(source_path.clone())
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
    println!("  rarag doctor [--socket <path>] [--json] [--dry-run-request]");
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
