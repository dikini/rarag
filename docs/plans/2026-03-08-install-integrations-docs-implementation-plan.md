# Install and Integration Docs Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

Task Registry ID: `2026-03-08-install-integrations-docs`

Goal: Provide a compact README plus dedicated install, user-systemd, and MCP integration documentation with practical examples.
Architecture: Keep technical behavior unchanged and implement this as a docs information architecture pass. Use a short README as a routing hub and move procedural detail to focused docs (`INSTALL.md`, ops, integrations). Maintain a tiered support model for integrations so docs stay realistic as client harnesses evolve.
Tech Stack: Markdown docs, existing Rust CLI/daemon/MCP help surfaces, repository doc quality scripts.
Template-Profile: tdd-strict-v1

## Instruction Priority

1. System-level constraints and safety policies.
2. Developer-level constraints and workflow policies.
3. This plan's execution and verification contracts.
4. Explicit plan updates recorded in this file.

## Execution Mode

- Mode: execute-with-checkpoints
- Default: execute-with-checkpoints unless user explicitly requests plan-only output.

## Output Contract

- Keep README short and navigational.
- Keep install instructions concrete and copy-pasteable.
- Distinguish tested vs best-effort integration tiers explicitly.

## Task Update Contract

- Do not claim client integration parity that is not actually tested.
- Every integration page must include support tier and `last_verified`.
- Keep the same command and contract terminology used by current binaries.

## Completion Gate

- Completion requires updated docs, task registry, and changelog.
- Completion requires `scripts/doc-lint.sh --changed --strict-new` and `scripts/check-fast-feedback.sh` passing.

## Model Compatibility Notes

- XML-style delimiter blocks are optional and treated as plain-text structure.
- Critical constraints are restated in plain language for model-robust adherence.

---

### Task 1: Restructure User-Facing Documentation Surface

**Files:**

- Create: `INSTALL.md`
- Create: `docs/ops/systemd-user.md`
- Create: `docs/integrations/README.md`
- Create: `docs/integrations/codex.md`
- Create: `docs/integrations/claude.md`
- Create: `docs/integrations/opencode.md`
- Create: `docs/integrations/goose.md`
- Create: `docs/integrations/kimi.md`
- Modify: `README.md`
- Modify: `docs/tasks/tasks.csv`
- Modify: `CHANGELOG.md`

**Preconditions**

- Existing README is too dense for first-time install and operations paths.
- No dedicated integration doc surface exists.

**Invariants**

- Runtime behavior and contracts remain unchanged.
- Help output from existing binaries remains canonical for command discovery.

**Postconditions**

- README is concise and points to the right docs.
- INSTALL and ops docs provide practical, user-focused flows.
- Integration docs exist with explicit tiering and freshness metadata.

**Tests (must exist before implementation)**

Unit:
- `doc-lint markdown shape check`

Invariant:
- `scripts/doc-lint.sh --changed --strict-new`

Integration:
- `scripts/check-fast-feedback.sh`

Property-based (optional):
- none

**Red Phase (required before code changes)**

Command: `scripts/doc-lint.sh --changed --strict-new`
Expected: no-op or fail before new docs exist and are compliant.

**Implementation Steps**

1. Rewrite README as a docs hub.
2. Add INSTALL guide with Debian-first tiered install paths and verification checks.
3. Add systemd user operations guide with unit examples and lifecycle commands.
4. Add integration docs index and per-client pages with tier and `last_verified`.
5. Update task registry and changelog.

**Green Phase (required)**

Command: `scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`
Expected: docs quality and policy checks pass.

**Refactor Phase (optional but controlled)**

Allowed scope: docs and changelog files listed above
Re-run: `scripts/doc-lint.sh --changed --strict-new && scripts/check-fast-feedback.sh`

**Completion Evidence**

- Preconditions satisfied
- Invariants preserved
- Postconditions met
- Unit, invariant, and integration checks passing
- CHANGELOG and task registry updated
