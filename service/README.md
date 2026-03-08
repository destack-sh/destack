# Service

First-party services for most things most software needs, including some Destack-specific background services.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`daemon`](daemon/README.md) | Experimental | Background service for watch mode, orchestration, and caching |
| [`lsp`](lsp/README.md) | Experimental | Language Server Protocol service implementation |

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
