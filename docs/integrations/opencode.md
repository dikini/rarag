# opencode Integration

- Support tier: `Tier 2` (best effort)
- Last verified: `2026-03-08` (template only)

## Model

Use opencode MCP registration to launch local `rarag-mcp`.

## Template Registration

```json
{
  "name": "rarag",
  "command": "rarag-mcp",
  "args": ["serve", "--config", "/home/<user>/.config/rarag/rarag.toml"]
}
```

## Verify

- MCP server starts without errors
- `rag_query` is visible in tool list
- one query succeeds for your repository

## Notes

Harness config format may change quickly. Keep this page aligned with current opencode MCP docs and update `last_verified` when validated.
