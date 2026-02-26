# Bridge

Destack bridge packages and integrations.
This layer provides publishable SDK surfaces, editor extensions, and host toolchain integrations.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `napi` | Node-API bindings package and crate. | [napi/README.md](napi/README.md) |
| `wasm` | WebAssembly bindings package and crate. | [wasm/README.md](wasm/README.md) |
| `typescript` | Unified client package for TypeScript and JavaScript. | [typescript/README.md](typescript/README.md) |
| `rust` | Rust client crate published as `destack`. | [rust/README.md](rust/README.md) |
| `python` | Python client package published as `destack`. | [python/README.md](python/README.md) |
| `bun` | Bun plugin and loader for `.ds` files. | [bun/README.md](bun/README.md) |
| `vite` | Vite plugin for Destack projects. | [vite/README.md](vite/README.md) |
| `vscode` | VS Code extension and language support. | [vscode/README.md](vscode/README.md) |
| `zed` | Zed extension integration. | [zed/README.md](zed/README.md) |

## Commands

Run these commands from the repository root.
Use `just bridge/test` as an alias for `just bridge/test-quick`.

```sh
just bridge/format
just bridge/check
just bridge/build
just bridge/test
just bridge/test-quick
just bridge/test-ci
just bridge/test-nightly
just bridge/test-release
just bridge/test-ide
just bridge/wasm-size
just bridge/publish --dry-run
just bridge/publish-zed --dry-run
```
