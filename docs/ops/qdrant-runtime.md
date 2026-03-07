# Qdrant Runtime Guide

This guide documents the runtime dependency for `rarag` when using the live vector path.

## Scope

Use this guide when:

- running `raragd` against a real vector store
- running live OpenAI plus Qdrant pre-merge checks
- operating a local developer instance that should preserve semantic vectors across daemon restarts

You do not need Qdrant for the default deterministic test path.

## Requirement

`rarag` expects a reachable Qdrant gRPC endpoint when the daemon is not started in test-memory mode.

Default config expectation:

- gRPC endpoint: `http://127.0.0.1:6334`
- collection: `rarag_chunks`

Qdrant commonly exposes:

- `6333`: HTTP API and dashboard
- `6334`: gRPC API used by the Rust client

## Local Single-Node Setup

Recommended local setup uses Docker with persisted storage.

```bash
mkdir -p "$HOME/.local/share/qdrant/storage"
docker pull qdrant/qdrant
docker run -d \
  --name rarag-qdrant \
  -p 127.0.0.1:6333:6333 \
  -p 127.0.0.1:6334:6334 \
  -v "$HOME/.local/share/qdrant/storage:/qdrant/storage" \
  qdrant/qdrant
```

Verification:

```bash
curl http://127.0.0.1:6333
```

## Podman Variant

```bash
mkdir -p "$HOME/.local/share/qdrant/storage"
podman run -d \
  --name rarag-qdrant \
  -p 127.0.0.1:6333:6333 \
  -p 127.0.0.1:6334:6334 \
  -v "$HOME/.local/share/qdrant/storage:/qdrant/storage:Z" \
  docker.io/qdrant/qdrant
```

## Config

Example `rarag.toml` section:

```toml
[qdrant]
endpoint = "http://127.0.0.1:6334"
collection = "rarag_chunks"
```

The daemon will create the configured collection if it does not already exist.

## Live Provider Environment

Live OpenAI plus Qdrant runs require:

```bash
export OPENAI_API_KEY='...'
export RARAG_LIVE_QDRANT_ENDPOINT='http://127.0.0.1:6334'
```

If you use the local environment file on this machine:

```bash
source ~/.config/sharo/daemon.env
export RARAG_LIVE_QDRANT_ENDPOINT='http://127.0.0.1:6334'
```

## Live Pre-Merge Check

The opt-in live end-to-end check uses the real OpenAI embeddings path and a real Qdrant endpoint.

```bash
scripts/check-live-rag-stack.sh
```

Expected prerequisites:

- Qdrant reachable at `RARAG_LIVE_QDRANT_ENDPOINT`
- `OPENAI_API_KEY` set in the environment

## Operational Notes

- Bind Qdrant to `127.0.0.1` for local development unless you explicitly need remote access.
- Do not expose Qdrant publicly without auth, TLS, and network controls.
- The deterministic test path does not exercise the live Qdrant dependency.
- The daemon test flag `--test-memory-vector-store` is for hermetic tests only and should not be used for normal runtime operation.

## Failure Modes

Common symptoms:

- `connection refused`
  - Qdrant is not running or the endpoint/port is wrong.
- `transport error`
  - gRPC endpoint is unreachable or blocked.
- empty semantic results after daemon restart
  - vectors were indexed into test-memory mode instead of real Qdrant.

## References

- Qdrant quick start: <https://qdrant.tech/documentation/quick-start/>
- Qdrant installation guide: <https://qdrant.tech/documentation/guides/installation/>
- Qdrant security guide: <https://qdrant.tech/documentation/guides/security/>
