# Changelog

All notable changes to this project will be documented in this file.

The format is based on Common Changelog:
<https://common-changelog.org/>

## Unreleased

### Added

- Added a repository RAG architecture spec, design note, and phased implementation plan for a Rust-first, worktree-aware hybrid retrieval system using Turso, Tantivy, Qdrant, `ra_ap_syntax`, and `rust-analyzer`.
- Added the Phase 1 Rust workspace skeleton with `rarag-core`, `raragd`, `rarag`, and `rarag-mcp`, plus bootstrap tests and toolchain configuration.
- Added initial application config and snapshot identity types with validation and JSON roundtrip coverage for worktree-aware indexing.
- Added shared app-config defaults and optional binary-specific config sections for CLI, daemon, and MCP settings.
- Added shared TOML config loading with deterministic search order and merge-on-default behavior in `rarag-core`.
- Added the Turso-backed metadata schema and snapshot store with indexing-run and query-audit recording.
- Added the first `ra_ap_syntax` structural chunker with workspace fixture coverage for symbols, tests, and oversized body-region splits.
- Added Tantivy indexing, prepared Qdrant point ingestion, and an OpenAI-compatible embedding request builder tied together through snapshot reindexing.
- Added workflow-aware retrieval modes with bounded neighborhood assembly, ranking evidence, and snapshot-local hybrid lookup.
- Added a checked-in OpenAI-compatible example config that references environment variables instead of secrets.

### Changed

- Initialized required project docs by resolving startup placeholders in `README.md` and `AGENTS.md`, and aligned security reporting guidance with repository issue-based intake.
- Made the OpenAI-compatible embedding client configurable for provider base URLs and endpoint paths, with the OpenAI default target aligned to `/v1/embeddings`.

### Fixed

### Removed

### Deprecated

### Security
