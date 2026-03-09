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

## 5. Start in One Project

In the project directory you want to work on:

```bash
cd /path/to/project-a
rarag index build --worktree "$PWD"
rarag query --worktree "$PWD" --mode understand-symbol --text "entry points"
```

Notes:

- Keep one `raragd` and one `rarag-mcp` process running.
- Reindex after meaningful code changes.

## 6. Work with Two or More Projects

Use the same running daemon/MCP server and index each project independently:

```bash
cd /path/to/project-a && rarag index build --worktree "$PWD"
cd /path/to/project-b && rarag index build --worktree "$PWD"
```

When querying, always pass the target `--worktree`:

```bash
rarag query --worktree /path/to/project-a --mode review-change --text "unsafe blocks"
rarag query --worktree /path/to/project-b --mode understand-symbol --text "config loader"
```

Operational rule:

- The active repository is selected by `--worktree` (CLI) or by the worktree/root path in MCP calls.

## 7. Work with Git Worktrees

Each git worktree is treated as a distinct repository root for indexing/querying.

Example:

```bash
cd /path/to/repo
git worktree add ../repo-feature feature/my-branch

rarag index build --worktree /path/to/repo
rarag index build --worktree /path/to/repo-feature

rarag query --worktree /path/to/repo --mode review-change --text "socket timeout"
rarag query --worktree /path/to/repo-feature --mode review-change --text "socket timeout"
```

Use this pattern when you need side-by-side context for `main` and a feature branch.

## 8. Optional Persistent Mode (`systemd --user`)

```bash
rarag service install
rarag service restart --service all
rarag status --worktree "$PWD" --json
```

## 9. Codex MCP Registration

Use:

- `docs/integrations/codex.md`

## 10. Optional Eval Replay

Run fixture-driven offline replay against the current daemon:

```bash
rarag eval replay \
  --fixtures tests/fixtures/eval/tasks.json \
  --worktree "$PWD" \
  --include-history \
  --history-max-nodes 4 \
  --json
```

## Troubleshooting

- `connection refused`: daemon not running or socket path mismatch.
- empty semantic results: index not built, credentials missing, or worktree mismatch.
- tools missing in client: stale MCP client session; restart the client process.

## Next Reading

- `INSTALL.md`
- `docs/ops/systemd-user.md`
- `docs/ops/lancedb-runtime.md`
- `docs/ops/optimization-rollout.md`
- `docs/integrations/README.md`
