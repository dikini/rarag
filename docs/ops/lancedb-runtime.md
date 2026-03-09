# LanceDB Runtime Guide

This guide documents local LanceDB runtime behavior for `rarag`.

## Scope

Use this guide when:

- running `raragd` with the default file-backed vector path
- running the opt-in live OpenAI end-to-end check
- operating a local developer instance that should preserve semantic vectors across daemon restarts

You do not need any external vector database for normal `rarag` runtime operation.

## Requirement

`rarag` stores vectors in a local LanceDB table configured in `rarag.toml`.

Default config expectation:

- DB root: `$XDG_STATE_HOME/rarag/lancedb`
- table: `rarag_chunks`
- distance metric: `cosine`

Example config section:

```toml
[lancedb]
db_root = "/home/user/.local/state/rarag/lancedb"
table = "rarag_chunks"
distance_metric = "cosine"
```

Supported `distance_metric` values:

- `cosine`
- `l2`
- `dot`

## Local Setup

Ensure the configured `db_root` directory is writable by the same user that runs `raragd`.

```bash
mkdir -p "$HOME/.local/state/rarag/lancedb"
```

No separate service process is required.

## Live Provider Environment

Live embeddings runs require:

```bash
export OPENAI_API_KEY='...'
```

If you use the local environment file on this machine:

```bash
source ~/.config/sharo/daemon.env
```

## Live Pre-Merge Check

The opt-in live check uses a real embeddings provider and local LanceDB persistence.

```bash
scripts/check-live-rag-stack.sh
```

Expected prerequisites:

- `OPENAI_API_KEY` set in the environment
- write permission for temporary runtime/state/cache paths

## Operational Notes

- Keep `lancedb.db_root` on local disk with enough free space for embeddings.
- Use `state_root`/`cache_root` placement that matches your backup/cleanup policy.
- The daemon test flag `--test-memory-vector-store` is for hermetic tests only and should not be used for normal runtime operation.

## Failure Modes

Common symptoms:

- `permission denied`
  - `lancedb.db_root` is not writable by the daemon user.
- empty semantic results after daemon restart
  - a different `lancedb.db_root` or table name is being used than the one that was indexed.
- no semantic vectors written
  - embeddings credentials are missing or invalid.
