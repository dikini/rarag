pub mod config;
pub mod metadata;
pub mod snapshot;
pub mod workspace {
    pub const BINARIES: [&str; 3] = ["rarag", "raragd", "rarag-mcp"];

    pub const fn default_socket_name() -> &'static str {
        "raragd.sock"
    }
}

#[cfg(test)]
mod tests {
    use crate::workspace;

    #[test]
    fn workspace_defaults_parse() {
        assert_eq!(workspace::default_socket_name(), "raragd.sock");
        assert!(workspace::BINARIES.contains(&"rarag"));
    }
}
