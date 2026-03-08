# Claude Integration

- Support tier: `Tier 1`
- Last verified: `2026-03-08` (Debian/Ubuntu)

## Model

Claude should launch `rarag-mcp` as a local MCP server process.

## Minimal Example

Command launched by Claude:

```bash
rarag-mcp serve --config ~/.config/rarag/rarag.toml
```

## Suggested Registration Shape

In your Claude MCP server config, use an entry equivalent to:

```json
{
  "name": "rarag",
  "command": "rarag-mcp",
  "args": ["serve", "--config", "/home/<user>/.config/rarag/rarag.toml"]
}
```

## Verify

1. Reload Claude MCP integrations.
2. Ensure `rag_examples` and `rag_blast_radius` are listed.
3. Call `rag_index_status` for your active worktree.

## Common Failures

- server launch failure:
  - binary not on `PATH`; use absolute command path
- no results:
  - repository not indexed or wrong `worktree_root`
- socket mismatch:
  - config path points to a different `rarag.toml` than expected
