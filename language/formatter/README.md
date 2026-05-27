# formatter

Source formatter for Destack covering JS, JSX, TS, TSX, and TS++, designed to follow established JS and TS formatting conventions on every _supported_ family (mostly stemming from Prettier, though technically the FIR is originally from Ruff since Prettier is implemented in JS (though even more technically Ruff's FIR _is_ based on Rome)).

Because Destack is designed for _modern strict TS(++)_, we do not fully support:
- tag-aware JSDoc semantics (like `@type` or `@satisfies` comments)
- legacy TypeScript angle-bracket assertions (like `<T>expr`, we don't even parse this)
- legacy import-attribute `assert` syntax
- "statement-like" decorators are on a full new line before the decorated item (instead of inline)
- unsupported legacy JS syntax like `with` (also see [language/parser](../../language/parser/README.md))

## Architecture

Formatting is a three-stage pipeline:

1. Parse source into the Destack AST plus raw trivia.
2. Walk the AST and emit FIR nodes.
3. Print FIR to text with width-aware grouping and line breaking.

## Testing

Run these commands from the repository root.

```sh
cargo check -p destack_formatter
cargo test -p destack_formatter
cargo test -p destack_test --test formatter
cargo test -p destack_test --test conformance-formatter
just language/check-quick
just language/check-full
```
