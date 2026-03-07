mod cli;
mod client;

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
    let config = rarag_core::config_loader::load_app_config(config_path.as_deref())
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
        Ok(command) => {
            if command.dry_run {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&command.request).expect("serialize request")
                );
                return;
            }

            match client::send_request(&command.socket_path, &command.request) {
                Ok(response) => {
                    if command.json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&response).expect("serialize response")
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
    println!(
        "  rarag status [--socket <path>] [--snapshot-id <id> | --worktree-root <path>] [--json]"
    );
    println!(
        "  rarag index --workspace-root <path> --repo-root <path> --worktree-root <path> --git-sha <sha> [--cargo-target <triple>] [--feature <csv>] [--cfg-profile <profile>] [--max-body-bytes <n>] [--socket <path>] [--json]"
    );
    println!(
        "  rarag query --mode <mode> --phase <phase> --text <query> [--symbol-path <path>] [--snapshot-id <id> | --worktree-root <path>] [--changed-path <path>]... [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
    println!(
        "  rarag blast-radius --phase <phase> --text <query> [--symbol-path <path>] [--snapshot-id <id> | --worktree-root <path>] [--changed-path <path>]... [--limit <n>] [--socket <path>] [--json] [--dry-run-request]"
    );
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
        rarag_core::daemon::DaemonResponse::Ack => println!("ack"),
        rarag_core::daemon::DaemonResponse::Error(err) => {
            println!("error={}", err.message);
        }
    }
}
