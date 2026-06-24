# Clients

Destack language clients for host languages and package ecosystems.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`typescript`](typescript/README.md) | Alpha | Primary language client package published as `@destack/language` |
| [`rust`](rust/README.md) | Experimental | Rust language client crate published as `destack` |
| [`python`](python/README.md) | Experimental | Python language client package published as `destack` |

The `napi` and `wasm` packages are TypeScript client backends.
They are maintained as implementation packages for `typescript`, not first-class client products.

## Commands

Run these commands from the repository root.

```sh
just client/format
just client/format-check
just client/lint
just client/build
just client/test
just client/check-quick
just client/check-full
just client/install-toolchain
just client/doctor-toolchain
just client/ensure-toolchain
just client/wasm-size
just client/publish --dry-run
```
