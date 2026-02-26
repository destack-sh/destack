# Service

Destack runtime and developer services.
This layer contains long-running daemons and language services.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `daemon` | Background service for watch mode, orchestration, and caching. | [daemon/README.md](daemon/README.md) |
| `lsp` | Language Server Protocol service implementation. | [lsp/README.md](lsp/README.md) |
| `lsp/server` | Shared JSON-RPC and transport server framework for LSP. | [lsp/server/README.md](lsp/server/README.md) |
| `lsp/types` | LSP type definitions used across server and clients. | [lsp/types/README.md](lsp/types/README.md) |

## Commands

Run these commands from the repository root.
Use `just service/test` as an alias for `just service/test-quick`.

```sh
just service/format
just service/check
just service/build
just service/test
just service/test-quick
just service/test-ci
just service/test-nightly
just service/test-release
```
