# Platform

Destack platform tools and integrations.
CLI, editor support, and build system plugins.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `cli` | Command-line interface (`destack` binary) | [cli/README.md](cli/README.md) |
| `lsp` | Language Server Protocol implementation | [lsp/README.md](lsp/README.md) |
| `vscode` | VS Code extension (syntax, themes, LSP client) | [vscode/README.md](vscode/README.md) |
| `daemon` | Background service for watch mode and caching | [daemon/README.md](daemon/README.md) |
| `bun` | Bun plugin and loader for `.ds` files | [bun/README.md](bun/README.md) |
| `vite` | Vite plugin for Destack projects | [vite/README.md](vite/README.md) |
| `zed` | Zed extension integration | [zed/README.md](zed/README.md) |

## Commands

```sh
just platform/cli     # build CLI
just platform/lsp     # build LSP server
just platform/vscode  # build VS Code extension
just platform/build   # build all platform crates
```

