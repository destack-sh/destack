# builtin

Builtin definitions are the language and host surface that the Destack toolchain ships with.
The compiler loads these according to the active target profile.

## Layers

The "builtins" are split into three layers:

- `intrinsic/`: internal language constructs that need to be "well-known" to the compiler and language toolchain, like marker traits, operator overloading.
- `language/`: ambient language features, like `Number`, `BigInt`, `Temporal`, `Promise`, including both typings and shims for `.js`/`.ts` targets and actual implementations for `.ds` targets.
- `library/`: host and platform libraries and integrations, like `platform:*` and `destack:*`, again including both typings and shims for other runtimes and actual implementations for our own runtime.

## Testing

Run these from the repository root.

```sh
cargo test -p destack_builtin
just language/test-specification
just language/test-query
just language/quick
```
