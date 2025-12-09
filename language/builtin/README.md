# Destack Built-ins

Language built-in definitions. Shipped with the language toolchain.

## Structure

```
src/
├── core/                   # Language primitives (always loaded)
│   ├── type.ds             # Type<T>, typeOf(), Field, Method
│   ├── operator.ds         # Add, Subtract, Compare, etc.
│   ├── primitive.ds        # int32, uint64, float64, etc.
│   └── iterator.ds         # Iterator, Iterable
│
├── lib/                    # Standard library definitions (loaded via `lib` config)
│   ├── es/                 # ECMAScript versions
│   │   ├── es5/
│   │   ├── es2015/
│   │   ├── ...
│   │   ├── es2024/
│   │   └── esnext/
│   │
│   ├── dom/                # Browser DOM APIs
│   │
│   └── worker/             # Web Worker APIs
│
└── index.ds

scripts/
└── sync-libs.sh            # Script to update lib definitions
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
