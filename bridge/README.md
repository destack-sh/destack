# Bridge

Destack bridge into and out of the existing universe.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`core`](core/README.md) | Experimental | Shared Rust bridge core used by language clients and bindings |
| [`capi`](capi/README.md) | Experimental | Minimal C ABI bridge surface for FFI language clients |
| [`typescript`](typescript/README.md) | Alpha | Primary runtime client package published as `@destack/runtime` |
| [`napi`](napi/README.md) | Experimental | Node-API bindings package and crate |
| [`wasm`](wasm/README.md) | Experimental | WebAssembly bindings package and crate |
| [`rust`](rust/README.md) | Experimental | Rust client crate published as `destack` |
| [`python`](python/README.md) | Experimental | Python client package published as `destack` |
| [`go`](go/README.md) | Experimental | Go client module intended for `go.destack.sh/destack` |
| [`dotnet`](dotnet/README.md) | Experimental | .NET client intended for NuGet as `Destack` |
| [`java`](java/README.md) | Experimental | Java client intended for Maven Central as `industries.symbol.destack:destack-java` |
| [`ruby`](ruby/README.md) | Experimental | Ruby client intended for RubyGems as `destack` |
| [`dart`](dart/README.md) | Experimental | Dart client intended for pub.dev as `destack` |
| [`elixir`](elixir/README.md) | Experimental | Elixir client intended for Hex as `destack` |
| [`swift`](swift/README.md) | Experimental | Swift client intended for Swift Package Manager as `Destack` |
| [`vscode`](vscode/README.md) | Experimental | VS Code extension and language support |
| [`zed`](zed/README.md) | Experimental | Zed extension integration |

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
