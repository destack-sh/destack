# Destack Built-ins

Language built-in definitions. Shipped with the language toolchain.

This crate provides:
- **Rust**: `destack_builtin` crate with `LanguageItem`, `PreludeItem`, and embedded `.ds` sources
- **Destack**: Core language primitives and library type definitions

## Structure

```
src/                        # Rust source (destack_builtin crate)

core/                       # Language primitives (always loaded)
├── operator/               # Operator overloading interfaces
├── reflection/             # Reflection system (Type<T>, decorators, etc.)
└── intrinsic/              # Compiler-provided features

lib/                        # Runtime library definitions (loaded via `lib` config)
├── es/                     # ECMAScript versions
├── dom/                    # Browser DOM APIs
└── worker/                 # Web Worker APIs
```

## What's Bundled vs External

**Bundled** (no npm package provides these):
- `core/` - Destack-specific primitives
- `lib/es/*` - ECMAScript standard library
- `lib/dom/` - Browser DOM APIs
- `lib/worker/` - Web Worker APIs

**External** (from node_modules):
- Node.js types → install `@types/node`
- Bun types → install `bun-types`
- Deno types → install `@types/deno`

The compiler automatically discovers types from node_modules when available.

## Configuration

Built-ins are loaded based on the `lib` array in `dsconfig.json`:

```json
{
  "compilerOptions": {
    "lib": ["es2024", "dom"]
  }
}
```

## Updating

Run the sync script to update lib definitions from TypeScript:

```bash
cd language/builtin
./scripts/sync-libs.sh
```

## Sources

- ES/DOM/Worker: TypeScript lib definitions (Apache 2.0)
