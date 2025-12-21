# codegen/js

JavaScript and TypeScript code generation.
Takes elaborated DIR and produces `.js` or `.ts` source files.

## Pipeline

```
DIR → JS AST → FIR Document → Source Text
      (lower)    (format)      (print)
```

1. **Lower** (`lower/`): Walk DIR, emit a simplified JS AST
2. **Format** (`format/`): Convert JS AST to FIR document nodes
3. **Print**: Use FIR printer to produce final source text

The intermediate JS AST (`tree/`) is simpler than the Destack AST.
It only has constructs that exist in JavaScript/TypeScript.

## What Gets Lowered

Most canonical DIR constructs map directly to JS/TS equivalents:

| Destack | JavaScript/TypeScript |
|---------|----------------------|
| `struct Point { x, y }` | `type Point = { x: number, y: number }` |
| `match (x) { ... }` | `switch` or chained `if` |
| `(a, b)` tuple | `[a, b]` array |
| `1..10` range | `Array.from(...)` or loop |
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
