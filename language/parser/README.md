# parser

Lexer and parser for Destack, TypeScript, and JavaScript.
Transforms source text into the shared Destack AST.

## Overview

The parser handles `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx` files (mostly) uniformly.
Destack syntax is a superset of (modern, strict) TypeScript, so valid TS/JS parses to a valid Destack AST subset.

### "Recursive Descent"

The parser uses "recursive descent" with operator precedence climbing.
(Why this deserves a special name is still unclear to me: it's just how you write a parser?)

Like most parsers, we use a lookahead and some context to make parsing decisions.
The parser tries to continue after errors to report multiple diagnostics per file.
We use synchronization points (`;`, `}`, keywords) to resync after malformed input.

### Tree Literals (TSX)

We support full JSX/TSX compatibility for "tree literals".
This is somewhat annoying, but we borrow some web-land tricks here to make this tractable with a scanner-ish approach, and unfortunately also do some unavoidable backtracking.

```tsx
// simple case
<Component prop={value}>
    <Child />
    {expression}
</Component>

// mildly annoying case
type T<S: string> = A<S>; 

// very annoying case
let Component = <Component<T<"button.press">> />.Child<T>; // what even is this?
```

## Testing

Run these from the repository root.

```sh
cargo test -p destack_parser
cargo test -p destack_test --test smoke -- --parser
just language/test-conformance
just language/check-quick
just language/check-full
```
