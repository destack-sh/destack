# Bridge

Destack bridge packages and integrations.
This layer provides publishable SDK surfaces, editor extensions, and host toolchain integrations.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `napi` | Node-API bindings package and crate. | [napi/README.md](napi/README.md) |
| `wasm` | WebAssembly bindings package and crate. | [wasm/README.md](wasm/README.md) |
| `core` | Shared Rust bridge core used by language clients and bindings. | [core/README.md](core/README.md) |
| `capi` | Minimal C ABI bridge surface for FFI language clients. | [capi/README.md](capi/README.md) |
| `typescript` | Runtime client package published as `@destack/runtime`. | [typescript/README.md](typescript/README.md) |
| `rust` | Rust client crate published as `destack`. | [rust/README.md](rust/README.md) |
| `python` | Python client package published as `destack`. | [python/README.md](python/README.md) |
| `go` | Go client module intended for `go.destack.sh/destack`. | [go/README.md](go/README.md) |
| `dotnet` | .NET client intended for NuGet as `Destack`. | [dotnet/README.md](dotnet/README.md) |
| `java` | Java client intended for Maven Central as `com.symbol.destack:destack-java`. | [java/README.md](java/README.md) |
| `ruby` | Ruby client intended for RubyGems as `destack`. | [ruby/README.md](ruby/README.md) |
| `dart` | Dart client intended for pub.dev as `destack`. | [dart/README.md](dart/README.md) |
| `elixir` | Elixir client intended for Hex as `destack`. | [elixir/README.md](elixir/README.md) |
| `swift` | Swift client intended for Swift Package Manager as `Destack`. | [swift/README.md](swift/README.md) |
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
just bridge/test-language-bridges
just bridge/toolchain-install
just bridge/toolchain-doctor
just bridge/toolchain-ensure
just bridge/test-nightly
just bridge/test-release
just bridge/test-ide
just bridge/wasm-size
just bridge/publish --dry-run
just bridge/publish-zed --dry-run
```
