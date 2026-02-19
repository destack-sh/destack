# Client

Destack client SDKs and bindings.
This layer provides publishable package and crate surfaces across JavaScript, Rust, Python, and WebAssembly.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `napi` | Node-API bindings package and crate. | [napi/README.md](napi/README.md) |
| `wasm` | WebAssembly bindings package and crate. | [wasm/README.md](wasm/README.md) |
| `typescript` | Unified client package for TypeScript and JavaScript. | [typescript/README.md](typescript/README.md) |
| `rust` | Rust client crate published as `destack`. | [rust/README.md](rust/README.md) |
| `python` | Python client package published as `destack`. | [python/README.md](python/README.md) |

## Commands

Run these commands from the repository root.

```sh
just client/build
just client/check
just client/test
just client/wasm-size
just client/publish-npm --dry-run
just client/publish-cargo --dry-run
just client/publish-pypi --dry-run
```
