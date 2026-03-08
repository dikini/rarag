# kimi Integration

- Support tier: `Tier 2` (best effort)
- Last verified: `2026-03-08` (template only)

## Model

Configure kimi to launch `rarag-mcp` as a local MCP server command.

## Template Registration

```json
{
  "name": "rarag",
  "command": "rarag-mcp",
  "args": ["serve", "--config", "/home/<user>/.config/rarag/rarag.toml"]
}
```

## Verify

- server registration succeeds
- MCP tool list includes `rag_reindex` and `rag_index_status`
- one status query executes successfully

## Notes

Kimi integration formats may evolve quickly; keep this page versioned with `last_verified` updates.
