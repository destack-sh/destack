# Destack Daemon

Destack background daemon service per workspace.

## Commands

Use CLI commands to manage daemon instances:

- `destack daemon serve` runs a foreground daemon server
- `destack daemon start` ensures a daemon is running
- `destack daemon status` reports daemon connectivity
- `destack daemon stop` requests shutdown

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_daemon

# clean check
just service/check-quick

# exhaustive check
just service/check-full
```
