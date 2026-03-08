# systemd User Services

This guide covers running `raragd` and `rarag-mcp` as user-level systemd services.

## Scope

Use this when you want persistent local services without root/system-level units.

Assumptions:

- binaries are installed and on `PATH`
- `~/.config/rarag/rarag.toml` exists
- runtime dependencies are configured (for example Qdrant for live vector runtime)

## Unit Locations

User unit files live in:

- `~/.config/systemd/user/`

Recommended units:

- `raragd.service`
- `rarag-mcp.service`

## Example Unit Files

`~/.config/systemd/user/raragd.service`:

```ini
[Unit]
Description=rarag daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=%h/.cargo/bin/raragd serve --config %h/.config/rarag/rarag.toml
Restart=on-failure
RestartSec=2
EnvironmentFile=-%h/.config/rarag/daemon.env

[Install]
WantedBy=default.target
```

`~/.config/systemd/user/rarag-mcp.service`:

```ini
[Unit]
Description=rarag MCP server
After=raragd.service
Requires=raragd.service

[Service]
Type=simple
ExecStart=%h/.cargo/bin/rarag-mcp serve --config %h/.config/rarag/rarag.toml
Restart=on-failure
RestartSec=2
EnvironmentFile=-%h/.config/rarag/daemon.env

[Install]
WantedBy=default.target
```

## Install and Enable

```bash
mkdir -p ~/.config/systemd/user
# place the two unit files shown above
systemctl --user daemon-reload
systemctl --user enable --now raragd.service
systemctl --user enable --now rarag-mcp.service
```

## Day-2 Operations

```bash
systemctl --user status raragd.service
systemctl --user status rarag-mcp.service
systemctl --user restart raragd.service
systemctl --user restart rarag-mcp.service
systemctl --user stop rarag-mcp.service
systemctl --user start rarag-mcp.service
```

Logs:

```bash
journalctl --user -u raragd.service -f
journalctl --user -u rarag-mcp.service -f
```

## Config Reload

Prefer daemon reload for config-only changes:

```bash
rarag daemon reload --json
```

Equivalent signal path:

```bash
systemctl --user kill -s HUP raragd.service
```

If reload fails, daemon keeps last known-good configuration.

## Health Checks

After unit start/restart:

```bash
rarag status --worktree "$PWD" --json
```

MCP tool visibility check:

```bash
rarag-mcp --list-tools
```

## Upgrade Flow

1. Upgrade/reinstall binaries.
2. Reload systemd metadata.
3. Restart services.
4. Re-run health checks.

```bash
systemctl --user daemon-reload
systemctl --user restart raragd.service
systemctl --user restart rarag-mcp.service
rarag status --worktree "$PWD" --json
```

## Disable / Remove

```bash
systemctl --user disable --now rarag-mcp.service
systemctl --user disable --now raragd.service
rm -f ~/.config/systemd/user/rarag-mcp.service
rm -f ~/.config/systemd/user/raragd.service
systemctl --user daemon-reload
```

## Common Failures

- socket path collision:
  - stop stale process and remove stale socket if needed
- daemon starts but retrieval is weak:
  - check Qdrant endpoint and embeddings configuration
- permission denied on runtime paths:
  - ensure user owns configured runtime/cache/state directories
- MCP service up but client cannot connect:
  - verify `mcp.socket_path` in `rarag.toml` and matching client config

## Security Notes

- keep services in user scope unless you explicitly need system-wide runtime
- keep secrets in env files (`EnvironmentFile=`) or environment managers, not unit files
- prefer localhost-only exposure and local sockets
