use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    PrintConfig,
    Serve(ServeConfig),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeConfig {
    pub socket_path: Option<PathBuf>,
    pub config_path: Option<PathBuf>,
    pub deterministic_embeddings: bool,
    pub memory_vector_store: bool,
}

pub fn parse_args(args: &[String]) -> Result<Command, String> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Command::Help);
    }
    if args.iter().any(|arg| arg == "--print-config") {
        return Ok(Command::PrintConfig);
    }
    if args.get(1).is_some_and(|arg| arg == "serve") {
        return Ok(Command::Serve(ServeConfig {
            socket_path: option_value(args, "--socket").map(PathBuf::from),
            config_path: option_value(args, "--config").map(PathBuf::from),
            deterministic_embeddings: args
                .iter()
                .any(|arg| arg == "--test-deterministic-embeddings"),
            memory_vector_store: args.iter().any(|arg| arg == "--test-memory-vector-store"),
        }));
    }

    Ok(Command::Help)
}

fn option_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].clone())
}
