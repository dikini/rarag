# Codex Integration

- Support tier: `Tier 1`
- Last verified: `2026-03-08` (Debian/Ubuntu)

## Model

Codex should connect to local `rarag-mcp` as a stdio MCP server process.

## Minimal Example

Use a local MCP server entry that launches:

```bash
rarag-mcp serve-stdio --config ~/.config/rarag/rarag.toml
```

## Suggested Registration Shape

Use your Codex MCP configuration file and register a server entry equivalent to:

```json
{
  "name": "rarag",
  "command": "rarag-mcp",
  "args": ["serve-stdio", "--config", "/home/<user>/.config/rarag/rarag.toml"]
}
```

## Verify

1. Restart Codex MCP session.
2. Confirm tools include `rag_query` and `rag_symbol_context`.
3. Run one smoke query against the current repo.

Service-side check:

```bash
rarag-mcp --list-tools
```

## Common Failures

- tools not visible:
  - stale Codex MCP session; reconnect
- daemon error responses:
  - check `raragd` status and socket path alignment
- request timeout:
  - confirm local machine is not under heavy I/O pressure and daemon is responsive
