# parser

Lexer and parser for Destack, TypeScript, and JavaScript.
Transforms source text into the [Destack AST](../ast/README.md).

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

## Annotations

The parser treats semantic annotations and layout trivia as different classes with different owners.
 1. Decorators and documentation are semantic attachments.
 2. Comments and blanks are layout trivia records.

**However**, we still parse documentation as regular trivia, and then post-hoc assert it into the main AST IR because that turned out to be much easier.
This happens via `Parser::parse()`'s call out to `attach_trivia()`.
(If a test or direct parser entrypoint bypasses `parse()`, it must call `attach_trivia()` before checking docs or trivia output. Decorators are fine without.)

### Trivia Stream

Comment and blank trivia are emitted in source order into split buffers:
 - Comment trivia is stored in `NodeTree::comment_trivia()`.
 - Blank trivia is stored in `NodeTree::blank_trivia()`.
 - Mixed source iteration uses `NodeTree::trivia_refs()`.

## Testing

Run these from the repository root.

### Parser-focused local loop

```sh
cargo test -p destack_parser
cargo test -p destack_test --test smoke -- --parser
```

### Shared test gates

```sh
just language/quick
just language/full
```

### Parser conformance coverage

```sh
cargo test --release -p destack_test --test parser-conformance
```

### Parser performance and fuzzing

```sh
just language/bench-parser
just language/bench-lexer
just language/fuzz-parser 300
just language/fuzz-lexer 300
```
