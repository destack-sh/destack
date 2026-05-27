# Service

First-party services for most things most software needs, including some Destack-specific background services.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`daemon`](daemon/README.md) | Experimental | Background service for watch mode, orchestration, and caching |

## Commands

Run these commands from the repository root.

```sh
just service/format
just service/format-check
just service/lint
just service/build
just service/test
just service/check-quick
just service/check-full
```
