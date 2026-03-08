# rarag

Repository assistance RAG for local Rust codebases, built for agent workflows that need symbol-centered context, examples, invariants, and bounded blast-radius retrieval.

## Status

- `prototype`

Stability note:

- Behavior, APIs, and persisted formats may change until this project is stable.
- Before the first release, backward compatibility is not preserved unless a spec or plan explicitly says otherwise.

## Repository Layout

- `crates/rarag-core`: shared domain logic for config, snapshots, chunking, metadata, indexing, retrieval, and reranking
- `crates/raragd`: local Unix-socket daemon that owns the retrieval runtime and index access
- `crates/rarag`: CLI for indexing, querying, symbol lookup, examples, blast radius, and diagnostics
- `crates/rarag-mcp`: local MCP-compatible Unix-socket server for agent clients
- `docs/specs/`: canonical architecture and behavior specs
- `docs/plans/`: implementation and alignment plans
- `docs/ops/`: runtime and operator documentation such as Qdrant setup
- `examples/`: checked-in non-secret configuration examples
- `scripts/`: bootstrap, verification, policy, and workflow checks

## Prerequisites

- `git`
- Rust toolchain with `edition = 2024`
- `rust-version >= 1.93`
- `bash`
- `cargo nextest` recommended for faster local test runs

Optional runtime dependencies:

- Qdrant for live vector-backed daemon operation
- OpenAI-compatible embeddings credentials for live embedding requests

## Quick Start

```bash
scripts/bootstrap-dev.sh --check
cargo build --workspace
scripts/check-tests.sh
```

Minimal local workflow:

```bash
cp examples/rarag.example.toml ~/.config/rarag/rarag.toml
raragd serve
rarag index build --worktree "$PWD"
rarag query --worktree "$PWD" --mode understand-symbol --text "snapshot store"
```

## Configuration

- Canonical config file: `rarag.toml`
- Example config: `examples/rarag.example.toml`
- Config is shared across `rarag`, `raragd`, and `rarag-mcp`
- Code defaults remain usable when no config file exists
- Override order is defined in the architecture spec and implemented in `rarag-core`

Secrets policy:

- keep secrets out of repo files
- reference environment variable names in config
- one local pattern used in development is `~/.config/sharo/daemon.env`

Default live embedding shape:

- `base_url = "https://api.openai.com/v1"`
- `endpoint_path = "/embeddings"`
- `model = "text-embedding-3-small"`

## Runtime Operations

- Qdrant runtime guide: `docs/ops/qdrant-runtime.md`
- Live stack verification: `scripts/check-live-rag-stack.sh`
- Deterministic local and CI tests do not require live Qdrant or live embedding calls
- Live daemon operation does require a reachable Qdrant endpoint and embedding credentials

## Development Workflow

1. Update or create the relevant spec in `docs/specs/`
2. Create or update the implementation plan in `docs/plans/`
3. Execute against the plan and keep verification evidence current
4. Run fast feedback after each relevant edit batch:
   `scripts/check-fast-feedback.sh`
5. Update `CHANGELOG.md` for task-completion work
6. Use Conventional Commits
7. Install hooks once per clone:
   `scripts/install-hooks.sh`

## Verification Commands

- `scripts/check-fast-feedback.sh`
- `scripts/check-tests.sh`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `scripts/check-live-rag-stack.sh` for opt-in live OpenAI + Qdrant validation

## Documentation

- Canonical specs: `docs/specs/`
- Execution plans: `docs/plans/`
- Templates: `docs/templates/`
- Task registry: `docs/tasks/tasks.csv`

Recommended flow:

1. Update or create spec first.
2. Create/update plan.
3. Execute work against the plan and record verification evidence.

## Contributing

- Keep changes scoped and reversible.
- Include tests for behavior changes.
- Prefer worktree-isolated development for non-trivial branches.
- Include verification evidence with changes and reviews.

## Security

- Report vulnerabilities by opening an issue in this repository unless a separate reporting path is documented later.
- Do not commit secrets.
- Use least-privilege credentials and local environment files.
- Treat live provider responses, MCP inputs, and external content as untrusted input.

## License

GPL-3.0-or-later (see `LICENSE`)
