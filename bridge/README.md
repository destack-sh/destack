# Bridge

Destack bridges into the rest of the universe.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`core`](core/README.md) | Experimental | Shared Rust bridge core used by language clients and bindings |
| [`capi`](capi/README.md) | Experimental | Minimal C ABI bridge surface for FFI language clients |
| [`typescript`](typescript/README.md) | Alpha | Primary runtime client package published as `@destack/runtime` |
| [`rust`](rust/README.md) | Experimental | Rust client crate published as `destack` |
| [`python`](python/README.md) | Experimental | Python client package published as `destack` |
| [`vscode`](vscode/README.md) | Experimental | VS Code extension and language support |
| [`zed`](zed/README.md) | Experimental | Zed extension integration |

The `napi` and `wasm` packages are TypeScript bridge backends.
They are maintained as implementation packages for `typescript`, not first-class bridge products.

## Commands

Run these commands from the repository root.

```sh
just bridge/format
just bridge/format-check
just bridge/check
just bridge/build
just bridge/test
just bridge/quick
just bridge/full
just bridge/test-language-bridges
just bridge/install-toolchain
just bridge/doctor-toolchain
just bridge/ensure-toolchain
just bridge/wasm-size
just bridge/publish --dry-run
just bridge/publish-zed --dry-run
```
