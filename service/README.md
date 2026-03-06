# Service

First-party services for most things most software needs, including some Destack-specific background services.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `daemon` | Background service for watch mode, orchestration, and caching. | [daemon/README.md](daemon/README.md) |
| `lsp` | Language Server Protocol service implementation. | [lsp/README.md](lsp/README.md) |

## Commands

Run these commands from the repository root.

```sh
just service/format
just service/format-check
just service/check
just service/build
just service/test
just service/quick
just service/full
```
