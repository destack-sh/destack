# fir

Formatting Intermediate Representation for pretty-printing code.
FIR is a document model that abstracts over layout decisions, letting you describe *what* should be printed and letting the printer figure out *how* to break lines.

## Background

This is based on [Ruff's formatter IR](https://github.com/astral-sh/ruff/tree/main/crates/ruff_formatter) (MIT), which itself builds on [Rome's formatter](https://github.com/rome/tools) and [Prettier's algorithm](https://github.com/prettier/prettier/blob/main/docs/technical-details.md).
The core idea traces back to Wadler's classic ["A prettier printer"](https://homepages.inf.ed.ac.uk/wadler/papers/prettier/prettier.pdf) paper.

The key insight: instead of directly emitting text, you build a document of abstract layout commands.
The printer then measures the document and decides where to break lines to fit within the configured line width.

## How It Works

You build a `Document` from `FormatNode`s, then print it.

```ds
// build a document describing an array
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

If it fits on one line: `[a, b]`

If it doesn't fit:
```
[
    a,
    b,
]
```

The `group` measures its content, and `soft_line_break_or_space` becomes either a space (flat) or a newline (expanded) depending on whether the group fits.

## Core Primitives

| Primitive | Description |
|-----------|-------------|
| `token("x")` | Literal ASCII text (no newlines) |
| `text("x")` | Arbitrary text (may contain unicode, newlines) |
| `space` | Single space character |
| `hard_line_break` | Always a newline |
| `soft_line_break` | Newline if group breaks, nothing if flat |
| `soft_line_break_or_space` | Newline if group breaks, space if flat |
| `group(...)` | Measures content, breaks if it doesn't fit |
| `indent(...)` | Increases indentation level |
| `block_indent(...)` | Hard line break, then indented content |
| `soft_block_indent(...)` | Soft line break, then indented content |

## Groups and Breaking

Groups are the core decision points.
A group starts in "flat" mode (try to fit on one line).
If the content exceeds the line width, the group switches to "expanded" mode and soft line breaks become real newlines.

Groups can be nested:

```ds
group(
    token("outer("),
    soft_block_indent(group(
        token("inner("),
        soft_block_indent(token("content")),
        token(")")
    )),
    token(")")
)
```

The outer group might break while the inner stays flat, or both might break.
The printer figures out the best layout.

## Best Fitting

Sometimes you want to try multiple layouts and pick the best one:

```ds
best_fitting(
    // try flat first
    [token("["), token("a, b, c"), token("]")],
    // fall back to expanded
    [token("["), block_indent(token("a,\nb,\nc,")), token("]")]
)
```

The printer tries variants in order and picks the first that fits.

## Fill

`Fill` packs as many items as possible per line:

```
[1, 2, 3,
 4, 5, 6,
 7, 8]
```

Instead of either all-flat or all-expanded, fill tries to maximize items per line.

## Beyond the Formatter

FIR isn't just for the main code formatter.
It's also used by:
- **JS codegen** (`codegen/js/`): generating TypeScript output from DIR
- **MIR text output** (`mir/src/format/`): pretty-printing MIR for debugging
- **Linter suggestions**: formatting suggested fixes

Having one document model means consistent formatting behavior across all code output.
