# Changelog

All notable changes to this project will be documented in this file.

The format is based on Common Changelog:
<https://common-changelog.org/>

## Unreleased

### Added

- Added a repository RAG architecture spec, design note, and phased implementation plan for a Rust-first, worktree-aware hybrid retrieval system using Turso, Tantivy, Qdrant, `ra_ap_syntax`, and `rust-analyzer`.
- Added the Phase 1 Rust workspace skeleton with `rarag-core`, `raragd`, `rarag`, and `rarag-mcp`, plus bootstrap tests and toolchain configuration.
- Added initial application config and snapshot identity types with validation and JSON roundtrip coverage for worktree-aware indexing.
- Added the Turso-backed metadata schema and snapshot store with indexing-run and query-audit recording.
- Added the first `ra_ap_syntax` structural chunker with workspace fixture coverage for symbols, tests, and oversized body-region splits.
- Added Tantivy indexing, prepared Qdrant point ingestion, and an OpenAI-compatible embedding request builder tied together through snapshot reindexing.

### Changed

- Initialized required project docs by resolving startup placeholders in `README.md` and `AGENTS.md`, and aligned security reporting guidance with repository issue-based intake.

### Fixed

### Removed

### Deprecated

### Security
