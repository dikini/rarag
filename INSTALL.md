# Installation Guide

This guide is for local user installs of `rarag` (daemon, CLI, MCP server) with a practical Debian/Ubuntu-first workflow.

## Platform Matrix

- `Ubuntu/Debian`: tested and documented in detail
- `Other Linux distros`: best effort

## What Gets Installed

`rarag` is a Rust workspace with three user-facing binaries:

- `rarag`: CLI client
- `raragd`: local Unix-socket daemon
- `rarag-mcp`: local MCP-compatible Unix-socket server

All three binaries read shared config from `rarag.toml`.

## Path A: Quick Local Run (No Persistent Services)

Use this path for local experimentation and development.

### 1. Prerequisites

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev ca-certificates curl git jq
```

Install Rust if needed:

```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```

### 2. Install Binaries from Checkout

From repository root:

```bash
cargo install --path crates/rarag --locked
cargo install --path crates/raragd --locked
cargo install --path crates/rarag-mcp --locked
```

Confirm binaries are available:

```bash
rarag --help
raragd --help
rarag-mcp --help
```

### 3. Configure

```bash
mkdir -p ~/.config/rarag
cp examples/rarag.example.toml ~/.config/rarag/rarag.toml
```

### 4. Start Daemon + MCP Server

Terminal 1:

```bash
raragd serve
```

Terminal 2:

```bash
rarag-mcp serve
```

### 5. Smoke Test

```bash
rarag status --worktree "$PWD" --json
rarag daemon reload --json
```

## Path B: Persistent User Services (`systemd --user`)

Use this path if you want `raragd` and `rarag-mcp` managed as user services.

1. Follow Path A steps 1-3 first.
2. Follow `docs/ops/systemd-user.md` to install and enable user units.
3. Verify services:

```bash
systemctl --user status raragd.service
systemctl --user status rarag-mcp.service
rarag status --worktree "$PWD" --json
```

## Path C: Advanced / Custom Layout

Use this path if you need non-default socket, cache, state, or collection settings.

1. Copy `examples/rarag.example.toml`.
2. Override runtime paths and service sections.
3. Pass explicit config path to every process:

```bash
raragd serve --config /path/to/rarag.toml
rarag-mcp serve --config /path/to/rarag.toml
rarag status --config /path/to/rarag.toml --worktree "$PWD" --json
```

## Runtime Dependencies

### Qdrant

`rarag` runtime retrieval expects a reachable Qdrant endpoint unless daemon test-memory mode is enabled.

Use the canonical Qdrant guide:

- `docs/ops/qdrant-runtime.md`

### Embeddings Provider

For live embeddings, configure an OpenAI-compatible provider in `rarag.toml` and set credentials via environment variables.

## Command Discovery (Porcelain for Subcommands)

Use help output as the canonical command surface:

```bash
rarag --help
rarag daemon reload --help
rarag index --help
rarag query --help
raragd --help
rarag-mcp --help
```

For scripts and automation, prefer `--json` where available.

## Troubleshooting

- `connection refused`:
  - daemon or MCP server is not running
  - socket path mismatch between config and running process
- `error loading config`:
  - validate TOML syntax and required fields
- empty retrieval output:
  - check indexing status and worktree/snapshot arguments
- live semantic retrieval missing:
  - verify Qdrant reachability and embeddings credentials

## Next Reading

- User service operations: `docs/ops/systemd-user.md`
- MCP integration docs: `docs/integrations/README.md`
- Architecture and contracts: `docs/specs/repository-rag-architecture.md`
