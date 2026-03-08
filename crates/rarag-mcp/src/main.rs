mod server;
mod tools;

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
        println!("binary=rarag-mcp");
        println!("socket_path={}", config.mcp_socket_path());
        return;
    }

    if args.iter().any(|arg| arg == "--list-tools") {
        for tool in tools::tool_definitions() {
            println!("{}", tool.name);
        }
        return;
    }

    if args.get(1).is_some_and(|arg| arg == "serve") {
        let socket_path = args
            .windows(2)
            .find(|window| window[0] == "--socket")
            .map(|window| std::path::PathBuf::from(&window[1]))
            .unwrap_or_else(|| std::path::PathBuf::from(config.mcp_socket_path()));
        let daemon_socket = server::daemon_socket_from_args(&args, config.daemon_socket_path());
        if let Err(err) = server::serve(&socket_path, &daemon_socket) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }

    if args.get(1).is_some_and(|arg| arg == "serve-stdio") {
        let daemon_socket = server::daemon_socket_from_args(&args, config.daemon_socket_path());
        if let Err(err) = server::serve_stdio(&daemon_socket) {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }

    print_help();
    std::process::exit(2);
}

fn print_help() {
    println!("rarag-mcp server");
    println!("Usage:");
    println!("  rarag-mcp [--help] [--config <path>] [--print-config] [--list-tools]");
    println!("  rarag-mcp serve [--socket <path>] [--daemon-socket <path>] [--config <path>]");
    println!("  rarag-mcp serve-stdio [--daemon-socket <path>] [--config <path>]");
}
