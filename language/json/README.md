# json

JSON/JSONC AST, parser, and formatter for Destack.

## Overview

This crate provides JSON handling with full trivia preservation for JSONC (JSON with Comments).
Unlike typical JSON parsers that discard formatting, this preserves comments and whitespace in the AST for faithful roundtripping.

The main components:

| Component | Description |
|-----------|-------------|
| `JsonDocument` | Root AST node with leading/trailing trivia |
| `JsonValue` | Null, Bool, Number, String, Array, Object |
| `JsonTrivia` | LineComment, BlockComment, Whitespace, Newline |
| `parse()` | Parse JSON/JSONC source into AST |
| `format_json()` | Pretty-print AST using FIR |
| `to_serde()` / `from_serde()` | Convert between AST and `serde_json::Value` |

## Trivia Preservation

Comments and whitespace attach to AST nodes via `trivia_before` and `trivia_after` fields.
This enables formatting to preserve comments in their original positions:

```jsonc
{
    // user settings
    "name": "test",
    /* count */ "value": 42
}
```

The formatter uses FIR (Formatting IR) for layout decisions, automatically breaking long arrays/objects across multiple lines when they exceed the line width.

## Format Options

| Option | Default | Description |
|--------|---------|-------------|
| `indent_style` | Space | Space or Tab |
| `indent_width` | 2 | Spaces per indent level |
| `line_width` | 80 | Target line width |
| `trailing_comma` | false | Add trailing commas (JSONC) |

## Testing

Run these from the repository root.

```sh
# focused local loop
cargo test -p destack_json
cargo test -p destack_repository
cargo test -p destack_workspace

# clean check
just language/check-quick

# exhaustive check
just language/check-full
```
