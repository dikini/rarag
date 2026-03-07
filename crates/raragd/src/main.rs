mod config;
mod server;
mod transport;

use config::{Command, parse_args};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args).unwrap_or(Command::Help) {
        Command::Help => print_help(),
        Command::PrintConfig => {
            let config_path = args
                .windows(2)
                .find(|window| window[0] == "--config")
                .map(|window| std::path::PathBuf::from(&window[1]));
            let config = rarag_core::config_loader::load_app_config(config_path.as_deref())
                .expect("load app config");
            println!("binary=raragd");
            println!("socket_path={}", config.daemon_socket_path());
        }
        Command::Serve(serve) => {
            let config = rarag_core::config_loader::load_app_config(serve.config_path.as_deref())
                .expect("load app config");
            if let Err(err) = server::serve(config, serve).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}

fn print_help() {
    println!("raragd daemon");
    println!("Usage:");
    println!("  raragd [--help] [--config <path>] [--print-config]");
    println!(
        "  raragd serve [--config <path>] [--socket <path>] [--test-deterministic-embeddings] [--test-memory-vector-store]"
    );
}
