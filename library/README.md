# Library

The Destack standard library.
Integrated packages for building full-stack applications with Destack.

## Packages

| Package | Description | Link |
|---------|-------------|------|
| `entity` | Core entity system, events, and paths | [entity/README.md](entity/README.md) |
| `test` | Testing utilities | [test/README.md](test/README.md) |
| `schema` | Shared schemas for cross-package contracts | [schema/README.md](schema/README.md) |
| `napi` | N-API bindings exposing Rust toolchain to JS | [napi/README.md](napi/README.md) |
| `wasm` | WebAssembly bindings exposing Rust toolchain to browser JS | [wasm/README.md](wasm/README.md) |

## Commands

Run these commands from the repository root.

```sh
just library/napi        # build napi bindings
just library/wasm        # build wasm bindings
just library/wasm-size   # canonical wasm size analysis with preset defaults
just library/test        # run tests
just library/fmt         # format code
just library/check       # lint and check code
```
