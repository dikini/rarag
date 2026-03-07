fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("raragd bootstrap binary");
        println!("Usage: raragd [--help] [--config <path>] [--print-config]");
        return;
    }

    let config_path = config_arg(&args);
    if args.iter().any(|arg| arg == "--print-config") {
        let config = rarag_core::config_loader::load_app_config(config_path.as_deref())
            .expect("load app config");
        println!("binary=raragd");
        println!("socket_path={}", config.daemon_socket_path());
        return;
    }

    println!("raragd bootstrap binary");
}

fn config_arg(args: &[String]) -> Option<std::path::PathBuf> {
    args.windows(2)
        .find(|window| window[0] == "--config")
        .map(|window| std::path::PathBuf::from(&window[1]))
}
