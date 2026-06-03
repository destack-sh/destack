# @destack/vscode

VS Code extension for the Destack language, library and platform.
The extension launches `destack lsp`, which is the editor adapter for the shared language daemon.

## Local Development

When developing across multiple worktrees, point VSCode to a shared Destack binary.
This avoids requiring a per-worktree `target/{debug,release}/destack` build.

Set these settings in your workspace or user settings.

```json
{
  "destack.server.command": "/absolute/path/to/destack",
  "destack.server.args": ["lsp"]
}
```

## Testing

Run these from the repository root.

```sh
# focused local loop
just bridge/test-vscode
just bridge/test-vscode-bridge

# clean check
just bridge/check-quick

# exhaustive check
just bridge/check-full
```
