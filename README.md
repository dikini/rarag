# rarag

scaffolding for agent assisted projects

## Status

Current status: `prototype`

Stability note:

- Behavior, APIs, and persisted formats may change until this project is stable.

## Repository Layout

Document the top-level shape so contributors can orient quickly.

- `scripts/`: automation, policy checks, and verification entrypoints
- `docs/`: specs, plans, templates, task registry, and sync governance docs
- `.github/workflows/`: CI policy checks and merge-result gate workflows

If this is a Rust workspace, prefer this pattern:

- `rarag-core`: shared domain logic and contracts
- `rarag-cli`: command-line entrypoint
- `rarag-daemon` (optional): long-running runtime/service

## Prerequisites

- `git`
- Rust toolchain with `edition = 2024` and `rust-version >= 1.93`
- `bash` 5.x+

Rust example:

- Rust toolchain with `edition = 2024`
- `rust-version >= 1.93`
- `cargo nextest` (optional but recommended)

## Quick Start

```bash
scripts/init-repo.sh --check
scripts/check-fast-feedback.sh
just verify
```

Rust baseline:

```bash
scripts/bootstrap-dev.sh --check
cargo build --workspace
scripts/check-tests.sh
```

## Example Config

- Example local config: `examples/rarag.openai.example.json`
- The checked-in example references environment variables only.
- Keep secrets outside the repo, for example in `~/.config/sharo/daemon.env`.
- For OpenAI embeddings, the documented default is:
  `base_url = "https://api.openai.com/v1"` and `endpoint_path = "/embeddings"`

## Development Workflow

Document the mandatory feedback loop and pre-commit expectations.

Example policy:

1. Run fast feedback after each relevant edit batch:
   `scripts/check-fast-feedback.sh`
2. Keep `CHANGELOG.md` updated for task-completion work.
3. Follow commit message convention:
   `Conventional Commits` (<https://www.conventionalcommits.org/en/v1.0.0/>)
4. Install hooks once per clone:
   `scripts/install-hooks.sh`

## Verification Commands

List deterministic commands used by local and CI verification.

- `scripts/check-fast-feedback.sh`
- `scripts/check-tests.sh`
- `scripts/check-merge-result.sh`

## Documentation

If this repo uses spec/plan governance, include explicit guidance.

- Canonical specs: `docs/specs/`
- Execution plans: `docs/plans/`
- Templates: `docs/templates/`

Recommended flow:

1. Update or create spec first.
2. Create/update plan.
3. Execute work against plan and record verification evidence.

## Contributing

- Open issues/PRs with clear problem statements and verification evidence.
- Keep changes scoped and reversible.
- Include tests for behavior changes.

## Security

- Report vulnerabilities by opening an issue in this repository.
- Do not commit secrets.
- Use scoped credentials and local environment files where applicable.

## License

GPL-3.0-or-later (see `LICENSE`)
