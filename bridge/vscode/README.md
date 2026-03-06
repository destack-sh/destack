# @destack/vscode

VS Code extension for the Destack language, library and platform.

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

Run grammar and VSCode bridge smoke tests.

```sh
bun run test:grammar
bun run test:bridge
```
