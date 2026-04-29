# generate/js

JS and TS target generation.
Takes elaborated DIR and produces final JavaScript oriented target outputs.

## Pipeline

```text
DIR → lower → script output → print
```

1. **Lower** (`lower/`): Walk DIR and build a simplified JS tree.
2. **Artifact** (`backend/artifact.rs`): Package one generated `ScriptOutput`.
3. **Print** (`backend/print.rs`, `format/`): Convert the JS tree to FIR and final source text.

The intermediate JS AST (`tree/`) is simpler than the Destack AST.
It only has constructs that exist in JavaScript/TypeScript.

Bundling, chunking, HTML/CSS coordination, asset linking, and final package assembly live in `language/compiler/src/link/script/`.
This crate only owns JavaScript oriented lowering, output construction, and compact printing.

## What Gets Lowered

Most canonical DIR constructs map directly to JS/TS equivalents:

| Destack | JavaScript/TypeScript |
|---------|----------------------|
| `struct Point { x, y }` | `type Point = { x: number, y: number }` |
| `match (x) { ... }` | `switch` or chained `if` |
| `(a, b)` tuple | `[a, b]` array |
| `for (const x of xs)` | Standard JS `for...of` |
| `Result<T, E>` | Union type with discriminant |
| `comptime { ... }` | Evaluated, result inlined |
| `&T`, `^T` | Just `T`, maybe cloned (ownership erased) |

Some features require runtime support or "polyfills".

## Runtime Type Identity (RTTI)

Type identity is demand-driven on JS targets. Most structs compile to plain objects
with no runtime tag. If a type needs runtime reflection (`typeOf`, `instanceof`,
`any`/`unknown`, or union discrimination), codegen emits a hidden symbol property
on instances:

```ts
const RTTI = Symbol.for("destack.rtti")

function makeUser(name: string) {
    const obj = { name }
    Object.defineProperty(obj, RTTI, {
        value: TYPEID_USER,
        enumerable: false,
    })
    return obj
}
```

This keeps JS semantics intact (no enumerable fields, no prototype changes) while
allowing runtime type checks without a global WeakMap.

## Output Formats

The JS codegen backend supports both `.js` and `.ts` output (and `.d.ts` for JavaScript + TypeScript):

- **TypeScript** (`.ts`): Preserves type annotations, interfaces, generics
- **JavaScript** (`.js`): Strips types, emits runtime-only code
- **TypeScript Declaration** (`.d.ts`): TypeScript declaration file
