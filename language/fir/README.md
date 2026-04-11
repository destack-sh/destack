# fir

Formatting Intermediate Representation for pretty-printing code-shaped text.
FIR is the document model shared by formatter-style output paths across the language stack.

The current FIR is based on [Ruff's formatter IR](https://github.com/astral-sh/ruff/tree/main/crates/ruff_formatter), which builds on the Rome formatter model and a width-aware grouping algorithm.
The core idea still traces back to Wadler's classic pretty-printing work.

## Model

FIR describes what should be printed while the printer decides where groups stay flat and where they break.
Formatters build a `Document` of `FormatNode`s and then print that document to text.

```ds
group(
    token("["),
    soft_block_indent(
        token("a"),
        token(","),
        soft_line_break_or_space(),
        token("b"),
    ),
    token("]")
)
```

That document prints as `[a, b]` when it fits.
It prints as the expanded multiline variant when the group breaks.

```text
[
    a,
    b,
]
```

## Core Primitives

The core FIR nodes are:

| Primitive | Description |
| --- | --- |
| `token("x")` | Literal ASCII text without newlines |
| `text("x")` | Arbitrary text that may contain Unicode or newlines |
| `space` | One space character |
| `hard_line_break` | Always emit a newline |
| `soft_line_break` | Emit a newline only when the current group breaks |
| `soft_line_break_or_space` | Emit a newline when the group breaks, otherwise a space |
| `group(...)` | Measure content and choose flat or expanded layout |
| `indent(...)` | Increase indentation without forcing a break |
| `block_indent(...)` | Break, then indent |
| `soft_block_indent(...)` | Soft break, then indent |

## Layout Selection

Groups are the main decision points.
A group starts in flat mode.
If the content no longer fits, the printer expands that group and turns soft breaks into real line breaks.

`best_fitting(...)` allows a formatter to try several document shapes in priority order.
`fill(...)` allows the printer to pack as many items as fit on each line instead of choosing only fully flat or fully expanded output.

## Usage

FIR is shared across several code-generation paths.
That shared document model keeps wrapping, indentation, and line-suffix behavior consistent.

Current major users are:

- the source formatter in [`language/formatter`](/Users/florian/symbol/destack-6/language/formatter)
- JavaScript generation paths under `language/compiler`
- MIR text formatting paths under `language/mir`
- formatter-style fix output in linting and tooling code

## Testing

Run these commands from the repository root.

```sh
cargo check -p destack_fir
cargo test -p destack_fir
just fmt
```
