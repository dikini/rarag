# rarag

Repository-assistance RAG for local Rust codebases. `rarag` provides a daemon, CLI, and MCP server for symbol-centered context retrieval, examples, and bounded blast-radius queries.

## Status

- `prototype`

Stability note:

- Behavior, APIs, and persisted formats may change until the project is stable.
- Before first release, backward compatibility is out of scope unless a spec or plan explicitly requires it.

## Quick Start

```bash
scripts/bootstrap-dev.sh --check
cargo build --workspace
scripts/check-tests.sh
```

Minimal local run:

```bash
cp examples/rarag.example.toml ~/.config/rarag/rarag.toml
raragd serve
rarag index build --worktree "$PWD"
rarag query --worktree "$PWD" --mode understand-symbol --text "snapshot store"
```

User-service porcelain:

```bash
rarag service install
rarag service restart --service all
rarag service reload
```

## Start Here

- Install and local setup: `INSTALL.md`
- User systemd services: `docs/ops/systemd-user.md`
- Qdrant runtime operations: `docs/ops/qdrant-runtime.md`
- MCP client integrations: `docs/integrations/README.md`
- Canonical behavior spec: `docs/specs/repository-rag-architecture.md`

## Repository Layout

- `crates/rarag-core`: shared config, snapshot, chunking, indexing, retrieval, reranking
- `crates/raragd`: local Unix-socket daemon runtime
- `crates/rarag`: CLI client
- `crates/rarag-mcp`: local MCP-compatible Unix-socket server
- `docs/`: specs, plans, ops docs, integration docs, templates
- `examples/`: non-secret config examples
- `scripts/`: bootstrap and verification helpers

## Development Workflow

1. Update/create spec in `docs/specs/` for non-trivial behavior changes.
2. Create/update an implementation plan in `docs/plans/`.
3. Run fast feedback after relevant edit batches:
   `scripts/check-fast-feedback.sh`
4. Update `CHANGELOG.md` for task-completion work.
5. Use Conventional Commits and install hooks:
   `scripts/install-hooks.sh`

## Security

- Do not commit secrets.
- Use least-privilege credentials and local env files.
- Treat MCP inputs and provider responses as untrusted input.

Dependency note:
- `CDLA-Permissive-2.0` in dependency scans currently comes from [`webpki-roots`](https://crates.io/crates/webpki-roots), which is pulled in by `reqwest` with `rustls` TLS support for HTTPS provider calls.
- Task Registry ID: `2026-03-08-dependency-refresh` tracks periodic dependency refresh and removal of temporary advisory ignores once upstream crates migrate.

## License

GPL-3.0-or-later (`LICENSE`)
