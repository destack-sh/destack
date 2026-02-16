# parser

Lexer and parser for Destack, TypeScript, and JavaScript.
Transforms source text into the [Destack AST](../ast/README.md).

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

### Error Recovery

The parser tries to continues after errors to report multiple diagnostics per file.
We use synchronization points (`;`, `}`, keywords) to resync after malformed input.

### Tree Literals (TSX)

We support full JSX/TSX compatibility for "tree literals".
This is very annoying, and we borrow some web-land tricks here (with a scanner-ish approach), and some unavoidable backtracking.

```tsx
<Component prop={value}>
    <Child />
    {expression}
</Component>
```

## Annotations

Destack treats semantic annotations and layout trivia as different classes with different owners.
Decorators and documentation are semantic attachments.
Comments and blanks are layout trivia records.

`Parser::parse()` always runs `attach_trivia()`.
If a test or direct parser entrypoint bypasses `parse()`, it must call `attach_trivia()` before checking docs or trivia output.

### Semantic Attachments

Decorators are parsed in legal productions and attached immediately to their explicit owners.
Documentation comments are normalized in `attach_trivia()` and attached as `Documentation` (`Doc`) semantic annotations.
Documentation attachment is prefix-oriented: the parser resolves the next attachable start owner, then uses enclosing fallback if needed.
Semantic attachment order on one owner is source-stable.

### Trivia Stream

Comment and blank trivia are emitted in source order into split buffers.
Comment trivia is stored in `NodeTree::comment_trivia()`.
Blank trivia is stored in `NodeTree::blank_trivia()`.
Mixed source iteration uses `NodeTree::trivia_refs()`.

Each trivia record stores:
- source span
- token seam metadata: `token_before`, `token_after`
- newline flags: leading and trailing
- `is_leading_candidate`
- normalized comment directive classification for comments

### Core Rules

#### parser does not place comments or blanks
The parser emits trivia facts only.
The formatter owns leading, trailing, dangling, and blank-line materialization decisions.

#### seam metadata is canonical parser output
`token_before` and `token_after` identify nearest attachable semantic seams for each trivia record.
Missing seams use `u32::MAX` sentinel encoding.

#### documentation stays semantic
Documentation comments are not emitted as comment trivia records.
They are attached as semantic docs with deterministic prefix-style ownership.

#### blank runs are explicit
Only newline runs that represent blank lines emit `Blank` trivia records.
Single newlines remain formatting whitespace, not blank trivia payload.

#### directives are lexed once
Directive classification is stored as `CommentDirective` on comment trivia.
Parser and formatter hot paths must not repeatedly rescan comment text for directive detection.

#### single ownership and source stability are invariant
Each semantic documentation attachment is attached once.
Each trivia record is emitted once.
Output ordering is deterministic and source-stable.

### Notes

The old parser-owned comment placement model and seam heuristics are removed.
Formatter correctness now depends on trivia cursor logic over this emitted stream.
