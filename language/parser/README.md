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
    inComptime: bool,     // inside comptime expressions
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

## Missing

- none
