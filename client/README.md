# Client

The TypeScript and JavaScript workspace client for Destack tools and applications.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`typescript`](typescript/README.md) | Alpha | Workspace client published as `@destack/language` |

The `napi` and `wasm` packages implement local TypeScript workspace transports.
They are not independent client products.

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
