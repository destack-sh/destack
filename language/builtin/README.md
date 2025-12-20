# Destack Built-ins

Language built-in definitions shipped with the toolchain.
These are automatically loaded by the compiler based on target configuration.

## Structure

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
│   Operators   │◄──────────────────│   Extensions    │◄──────────────────│  Concrete Types │
│   & Traits    │   imports from    │   & Utilities   │   imports from    │  (per target)   │
│               │                   │                 │                   │                 │
│  (universal)  │                   │  (universal API)│                   │ Array, Map, ... │
└───────────────┘                   └─────────────────┘                   └─────────────────┘
```

## Layers

### `core/` — Language Primitives

**Universal, compiler-known.** Always loaded, same on all targets:
- **Operator interfaces**: `Add`, `Eq`, `Compare`, `Index`, `Iterator`, etc.
- **Error handling**: `Result`, `Try`
- **Reflection**: `Type`, `typeOf`

The compiler has special knowledge of `core/` types (language items, intrinsics).
Desugaring like `a + b` → `a.add(b)` and `foo()?` → early return depends on `core/`.

```
// core/operator/arithmetic.ds
newtype interface Add<T, R = Self> {
    add(other: T): R
}

// core/result.ds
newtype Result<T, E> = Ok<T> | Err<E>
```

### `std/` — Universal Extensions

Adds Destack-specific functionality to types provided by `lib/` (e.g., `Array<T>`).

```
// std/array.ds - extensions that work on Array<T>
extension<T> for Array<T> {
    /// Sum all elements (not in JS Array)
    sum(): T where T: Add<T> {
        this.reduce((a, b) => a + b)
    }

    /// Get first element or null (ergonomic helper)
    first(): T | null {
        if (this.length > 0) { this[0] } else { null }
    }

    /// Group by key (not in JS Array)
    groupBy<K>(f: (T) => K): Map<K, Array<T>> { /* ... */ }
}
```

For operations that need different implementations per target, we use `comptime if`:

```
// std/array.ds
extension<T> for Array<T> {
    sort(cmp: (T, T) => Ordering): Array<T> {
        comptime if (target.isJS) { // nocheckin: `comptime if (..)`? or `if (comptime ..)`?
            // use JS sort
            this.toSorted((a, b) => cmp(a, b).toInt())
        } else {
            // native quicksort implementation
            quicksort(this, cmp)
        }
    }
}
```

### `lib/` — Target-specific Extensions

`lib/` is where concrete types like `Array<T>`, `Map<K,V>`, `Set<T>`, `Promise<T>` are actually defined per target (along with some target-specific stuff).

| Directory | Provides | When Loaded |
|-----------|----------|-------------|
| `lib/es/` | `Array`, `Map`, `Set`, `Promise`, `Object`, `Symbol`, etc. | JS targets |
| `lib/dom/` | `Window`, `Document`, DOM APIs | `runtime: browser` |
| `lib/node/` | `Buffer`, `fs`, `path`, Node.js APIs | `runtime: node` |
| `lib/worker/` | `WorkerGlobalScope`, Web Worker APIs | `runtime: worker` |


## Relationship to `library/`

The `language/builtin/` directory is for **language-level** primitives that the compiler ships.

The `library/` directory (at repo root) contains **application-level** packages:
- `@destack/schema` — Schema validation
- `@destack/ui` — UI framework
- `@destack/entity` — Entity system
- etc.

These are regular packages that import from `@destack/core` and `@destack/std`.
The compiler has no special knowledge of them.

```
language/builtin/     →  Compiler ships this, has special knowledge
library/              →  First-party packages, no compiler magic
```
