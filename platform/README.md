# Platform

Destack platform tools and integrations.
CLI, editor support, and build system plugins.

## Components

| Component | Description |
|-----------|-------------|
| `cli` | Command-line interface (`destack` binary) |
| `lsp` | Language Server Protocol implementation |
| `vscode` | VS Code extension (syntax, themes, LSP client) |
| `daemon` | Background service for watch mode and caching |
| `bun` | Bun plugin and loader for `.ds` files |
| `vite` | Vite plugin for Destack projects |
| `editor` | Editor-agnostic utilities |

## Commands

```sh
just platform/cli     # build CLI
just platform/lsp     # build LSP server
just platform/vscode  # build VS Code extension
just platform/build   # build all platform crates
```

