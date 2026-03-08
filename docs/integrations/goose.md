# goose Integration

- Support tier: `Tier 2` (best effort)
- Last verified: `2026-03-08` (template only)

## Model

Register `rarag-mcp` as a local MCP server process in goose.

## Template Registration

```json
{
  "name": "rarag",
  "command": "rarag-mcp",
  "args": ["serve", "--config", "/home/<user>/.config/rarag/rarag.toml"]
}
```

## Verify

- goose starts MCP server cleanly
- `rag_symbol_context` is listed
- a symbol-context query returns items

## Notes

Treat this as a baseline template. If goose MCP schema changes, update this page and `last_verified`.
