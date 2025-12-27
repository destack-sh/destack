# parser

Lexer and parser for Destack, TypeScript, and JavaScript.
Transforms source text into the Destack AST.

## Overview

The parser handles `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx` files uniformly.
Destack syntax is a superset of TypeScript, so valid TS/JS parses to a valid AST subset.
(Again, see the conformance tests for more details.)

### Two-Phase Parsing

Unlike most TS parsers, we separate the lexing and parsing phases to preserve CST-style information for annotations like decorators and comments better:
1. **Lexer** (`lex/`): Converts source text to tokens
2. **Parser** (`parse/`): Builds AST from token stream via recursive descent

There are tradeoffs: the lexer must be somewhat context-sensitive to handle template strings and tree literals.
But this lets the core parser focus on "main constructs" while annotations like docs and decorators just work automatically.

### "Recursive Descent"

The parser uses recursive descent with operator precedence climbing.
(Why this deserves a special name is unclear: it's just how you write a parser.)

Like most parsers, we use a lookahead and some context to make parsing decisions.
Many syntactic ambiguities in TypeScript (and some more in Destack) require context tracking:

```ds
struct ParserOptions {
    inStatic: bool,       // inside <...> generic args
    inType: bool,         // parsing a type expression
    inTreeLiteral: bool,  // inside TSX/tree literal
    inMatchCase: bool,    // => means match arm, not lambda
    // ... many more
}
```

We still need some lookaheads and speculative parsing unfortunately, but the fat context struct helps a lot.

### Error Recovery

The parser tries to continues after errors to report multiple diagnostics per file.
We use synchronization points (`;`, `}`, keywords) to resync after malformed input.

### Tree Literals (TSX)

Full JSX/TSX compatibility for "tree literals":

```
<Component prop={value}>
    <Child />
    {expression}
</Component>
```

The lexer maintains a state stack to handle nested elements and expression containers (`{...}`).

## Layout

| Path | Purpose |
|------|---------|
| `lex/` | Tokenizer: source text → tokens |
| `parse/` | Parser: tokens → AST |
| `tests/` | Parser unit tests |
| `fuzz/` | Fuzzing harness (see `fuzz/README.md`) |

---

## Pending

None.

## Missing

- Conditional types: parse `T extends U ? X : Y` in `parse/type.rs` with a dedicated `eat_type_conditional` that binds tighter than `?:` in value space but looser than unions/intersections
- Infer bindings: parse `infer T` and `infer T extends U` as dedicated type nodes, scoped only inside conditional types
- Mapped types: parse `{ [K in keyof T as ...]-?: T[K] }` with `readonly`/`-readonly` and `?`/`-?` modifiers, plus key remapping after `as`
- Type operators and queries: ensure `keyof`, `readonly`, `unique`, and `typeof` work in type contexts even when value parsing would otherwise take precedence
- Indexed access types: parse `T[K]` as a type-level index (distinct from value indexing), including chained access `T[K][P]`
- Construct signature types: parse `new (...) => T` and `abstract new (...) => T` into function signatures with `FunctionMode::New` and `FunctionAbstraction::Abstract`
- Template literal types: parse `` `${K}` `` as type templates with type spans, not value `Argument`s
- `this` types/parameters: parse `this` as a type expression and allow `this: T` in parameter lists
- Type predicates/asserts: parse `x is T`, `asserts x is T`, `asserts this is T` as type expressions for return types
- Import types: parse `import("mod").Type` and `import("mod")` into a dedicated type-import expression rather than a value import
- Integration plan: extend `parse/type.rs` for type-only forms, then wire `parse/expression.rs` to route type contexts to these parsers without leaking value ambiguity
- Keep `TypeUnary`/`TypeBinary` parsing for operator-like constructs (`readonly`, `typeof`, `keyof`, `as`, `is`, `extends`, `implements`, etc.) and keep `infer`/predicates as dedicated forms
- Comptime note: add `ParserOptions.in_comptime` and allow type-only parsing when unambiguous, otherwise require an explicit `type` prefix inside `comptime` blocks
- Tuple elements: parse `readonly`, `?`, and `...` modifiers in tuple types and retain them in the AST (likely via `BindingModifier`)
- `this` parameters: parse `this: T` in function signatures and enforce it as the first parameter in type contexts
- Call/construct signatures in type literals: parse `() => T`, `new (...) => T`, and `abstract new (...) => T` as signature nodes rather than fields
- Index signatures: parse `readonly [k: string]: T` and preserve optional/readonly modifiers on the resulting property
- Intrinsic types: parse the `intrinsic` keyword in type positions (used by TS lib utility types)
