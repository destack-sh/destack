# formatter

Source formatter for Destack covering JS, JSX, TS, TSX, and TS++.
The formatter is designed to follow "industry standard formatting", that is, to behave equivalent to Prettier / oxfmt on every _supported_ JS and TS family.

Because Destack is designed for _modern strict TS(++)_, we do not fully support:
- JSDoc semantics (like `@type` comments)
- legacy TypeScript angle-bracket assertions (like `<T>expr`, we don't even parse this)
- legacy import-attribute `assert` syntax
- .. and any of the other legacy JS stuff like `with` (also see [language/parser](../../language/parser/README.md))

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
just fmt
```
