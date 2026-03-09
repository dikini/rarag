# User/Ops Quickstart

This page is the shortest path to a working local `rarag` setup for operators and end users.

## 0. Prerequisites

Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev ca-certificates curl git jq
```

Rust (if missing):

```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```

## 1. Install Binaries

From repository root:

```bash
cargo install --path crates/rarag --locked
cargo install --path crates/raragd --locked
cargo install --path crates/rarag-mcp --locked
```

## 2. Configure

```bash
mkdir -p ~/.config/rarag
cp examples/rarag.example.toml ~/.config/rarag/rarag.toml
```

## 3. Start Services (Local Session)

Terminal 1:

```bash
raragd serve
```

Terminal 2:

```bash
rarag-mcp serve-stdio --config ~/.config/rarag/rarag.toml
```

## 4. Validate in CLI

Terminal 3:

```bash
rarag index build --worktree "$PWD"
rarag query --worktree "$PWD" --mode understand-symbol --text "snapshot store"
rarag-mcp --list-tools
```

Expected:

- `rarag query` returns candidates for the current worktree.
- `rarag-mcp --list-tools` prints `rag_query`, `rag_symbol_context`, and related tools.

## 5. Optional Persistent Mode (`systemd --user`)

```bash
rarag service install
rarag service restart --service all
rarag status --worktree "$PWD" --json
```

## 6. Codex MCP Registration

Use:

- `docs/integrations/codex.md`

## Troubleshooting

- `connection refused`: daemon not running or socket path mismatch.
- empty semantic results: index not built, credentials missing, or worktree mismatch.
- tools missing in client: stale MCP client session; restart the client process.

## Next Reading

- `INSTALL.md`
- `docs/ops/systemd-user.md`
- `docs/ops/lancedb-runtime.md`
- `docs/integrations/README.md`
