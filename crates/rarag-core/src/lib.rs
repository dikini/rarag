pub mod chunking;
pub mod config;
pub mod config_loader;
pub mod daemon;
pub mod embeddings;
pub mod indexing;
pub mod metadata;
pub mod retrieval;
pub mod semantic;
pub mod snapshot;
pub mod unix_socket;
pub mod worktree;
pub mod workspace {
    pub const BINARIES: [&str; 3] = ["rarag", "raragd", "rarag-mcp"];

    pub const fn default_socket_name() -> &'static str {
        "raragd.sock"
    }

    pub const fn default_mcp_socket_name() -> &'static str {
        "rarag-mcp.sock"
    }
}

#[cfg(test)]
mod tests {
    use crate::workspace;

    #[test]
    fn workspace_defaults_parse() {
        assert_eq!(workspace::default_socket_name(), "raragd.sock");
        assert_eq!(workspace::default_mcp_socket_name(), "rarag-mcp.sock");
        assert!(workspace::BINARIES.contains(&"rarag"));
    }
}
