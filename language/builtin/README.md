# builtin

Builtin definitions are the language and host surface that the toolchain ships with.
The compiler loads these according to the active target profile.

## Layers

Builtins are split into three layers.

- `intrinsic/` is the compiler and runtime substrate.
  This is where operator contracts, control-flow shapes, reflection descriptors, the prelude, and other implementation-known primitives live.
- `language/` is the language-level builtin environment.
  This is the home for builtin types and language-standard surfaces that Destack may provide directly on Destack-owned runtimes.
- `library/` is the host and compatibility layer.
  This is where `platform:*`, `destack:*`, web-compatible surfaces, and host-specific compatibility libraries live.

`intrinsic/` is always part of the language.
`language/` and `library/` are selected by target profile and host capabilities.

## Roles

The important split inside `library/` is semantic, not just organizational.

- `platform:*` is the raw substrate.
  These modules expose low-level host authority and runtime bindings.
- `destack:*` is the primary portable systems surface.
  This is the layer our own renderer and most higher-level libraries should usually target.
- host compatibility libraries such as DOM, Node, Deno, Bun, and worker surfaces remain separate and explicit.
  They exist to model existing environments, not to replace `destack:*`.

This keeps the builtin package honest.
The low-level substrate stays explicit.
The portable runtime surface stays principled.
Compatibility surfaces stay compatibility surfaces.

## Ambient And Explicit

Some builtin libraries are ambient.
They inject globals or declarations without an explicit import.

Other builtin libraries are explicit.
They only exist when code imports them directly.

That distinction is driven by builtin metadata.
It is not inferred from the directory name alone.

## Builtin And Standard Library

`language/builtin/` is not the whole standard library.
It is the compiler-shipped language and host surface.

The rich application-facing standard library lives in the top-level `library/` tree.
That higher-level library stack should usually target `destack:*`, not raw `platform:*`.

## Updating Compatibility Libraries

TypeScript compatibility sources are fetched with `language/builtin/fetch.py`.
The pinned upstream version lives there as well.

## Testing

Run these from the repository root.

```sh
cargo test -p destack_builtin
just language/test-specification
just language/test-query
just language/quick
```
