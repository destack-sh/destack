# builtin

Language built-in definitions shipped with the toolchain.
The compiler automatically loads these based on target configuration.

## Overview

Builtins are organized into three layers, each building on the previous:

```
                          ┌─────────────────────────────────────────┐
                          │              User Code                  │
                          │         (imports from all layers)       │
                          └───────────────────┬─────────────────────┘
                                              │
        ┌─────────────────────────────────────┼─────────────────────────────────────┐
        │                                     │                                     │
        ▼                                     ▼                                     ▼
┌───────────────┐                   ┌─────────────────┐                   ┌─────────────────┐
│    core/      │                   │      std/       │                   │      lib/       │
│               │                   │                 │                   │                 │
│  "Intrinsics" │◄──────────────────│  "Extensions"   │◄──────────────────│"Target-specific"│
│               │   imports from    │   "Utilities"   │   imports from    │                 │
│               │                   │                 │                   │                 │
│  (universal)  │                   │  (universal)    │                   │    (per target) │
└───────────────┘                   └─────────────────┘                   └─────────────────┘
```

## Layers

### `core/` - Language Primitives

Universal, compiler-known types.
Always loaded, same on all targets.
The compiler has special knowledge of `core/` types.
Desugaring like `a + b` → `a.add(b)` and `foo()?` → early return depends on these definitions.

What's in `core/`:
- **Operator interfaces**: `Add`, `Eq`, `Compare`, `Index`, `Iterator`, etc.
- **Error handling**: `Result`, `Try`, `Ok`, `Err`
- **Reflection**: `Type`, `typeOf`

```
// core/operator/arithmetic.ds
newtype interface Add<T, R = this> {
    add(other: T): R
}

// core/result.ds
newtype Result<T, E> = Ok<T> | Err<E>
```

### `std/` - Universal Extensions

Adds Destack-specific functionality, sometimes attaching to types or re-defining types from `lib/`.
These are extensions that work the same on all targets (ideally).
The standard library is loaded for all profiles but is not ambient; modules must be imported.

```
// std/array.ds
extension<T> for Array<T> {
    /// Sum all elements (not in JS Array)
    sum(): T where T: Add<T> {
        this.reduce((a, b) => a + b)
    }

    /// Get first element or null
    first(): T | null {
        if (this.length > 0) { this[0] } else { null }
    }

    /// Group by key (not in JS Array)
    groupBy<K>(f: (T) => K): Map<K, Array<T>> { /* ... */ }
}
```

For operations that need different implementations per target, we use `if (comptime ..)`:

```
extension<T> for Array<T> {
    sort(cmp: (T, T) => Ordering): Array<T> {
        if (comptime target.isJS) {
            this.toSorted((a, b) => cmp(a, b).toInt())
        } else {
            quicksort(this, cmp)
        }
    }
}
```

### `lib/` - Target-Specific Definitions

This is where concrete types like `Array<T>`, `Map<K,V>`, `Set<T>`, `Promise<T>` are actually defined.
Different targets get different implementations.

| Directory | Provides | When Loaded |
|-----------|----------|-------------|
| `lib/es/` | `Array`, `Map`, `Set`, `Promise`, `Object`, `Symbol`, etc. | JS targets |
| `lib/dom/` | `Window`, `Document`, DOM APIs | `runtime: browser` |
| `lib/node/` | `Buffer`, `fs`, `path`, Node.js APIs | `runtime: node` |
| `lib/deno/` | `Deno`, `Deno.fs`, Deno APIs | `runtime: deno` |
| `lib/bun/` | `Bun`, `Bun.spawn`, Bun APIs | `runtime: bun` |
| `lib/worker/` | `WorkerGlobalScope`, Web Worker APIs | `runtime: worker` |

Runtime-specific libs can also be versioned (e.g., `node.v22`, `deno.v2.6`, `bun.v1.3`).
Targets with `runtimeVersion` select the matching versioned lib.
`runtimeVersion: "latest"` (or omitted) uses the default alias shipped on disk.

## Builtin vs Library

There is some overlap between "builtin" and "library" since the whole stack is intended to be well integrated.
Conceptually, the `language/builtin/` stuff is for language-level primitives that the compiler ships and needs to know about.
Everything else is a library that the compiler doesn't need to know or assume anything about (ideally).

## Updating builtin libs

TypeScript lib sources are fetched with `language/builtin/fetch.py`.
The pinned TypeScript version lives in `language/builtin/fetch.py` and should be updated manually.
