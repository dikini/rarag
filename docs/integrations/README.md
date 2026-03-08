# MCP Integrations

This section documents how to connect agent harnesses to the local `rarag-mcp` server.

## Support Tiers

- `Tier 1`: actively tested on Debian/Ubuntu and maintained as canonical integration docs
- `Tier 2`: best-effort templates; config syntax may drift as client harnesses evolve

Current matrix:

- Codex: Tier 1
- Claude: Tier 1
- opencode: Tier 2
- goose: Tier 2
- kimi: Tier 2

## Integration Prerequisites

- `raragd` is running
- `rarag-mcp` is running
- socket path in `rarag.toml` is known (default from config)

Quick checks:

```bash
rarag status --worktree "$PWD" --json
rarag-mcp --list-tools
```

## Canonical Client Pages

- `docs/integrations/codex.md`
- `docs/integrations/claude.md`
- `docs/integrations/opencode.md`
- `docs/integrations/goose.md`
- `docs/integrations/kimi.md`

## Freshness Policy

Every integration page includes:

- support tier
- `last_verified` date
- minimal config
- verification checklist
- known failure modes

When client harness config changes, update the page and `last_verified` immediately.
