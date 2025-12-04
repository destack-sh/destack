# Library

The Destack standard library.
Integrated packages for building full-stack applications with Destack.

## Packages

| Package | Description |
|---------|-------------|
| `entity` | Core entity system, events, and paths |
| `telemetry` | Logging, metrics, and tracing |
| `ui` | UI primitives, input handling, styling |
| `web` | Web platform utilities |
| `test` | Testing utilities |
| `napi` | N-API bindings exposing Rust toolchain to JS |

## Commands

```sh
just library/napi   # build napi bindings
just library/test   # run tests
just library/fmt    # format code
just library/lint   # lint code
```

