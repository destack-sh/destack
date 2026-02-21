# Platform

Destack platform tools and integrations.
CLI, editor support, and build system plugins.
Language bindings and embeddable SDKs live in [../client](../client/README.md).

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

Run these commands from the repository root.
Use `just platform/test` as an alias for `just platform/test-quick`.

```sh
just platform/format
just platform/check
just platform/build
just platform/build-vscode
just platform/build-zed
just platform/test
just platform/test-quick
just platform/test-ci
just platform/test-nightly
just platform/test-release
just platform/test-ide
just platform/generate-schema
```
