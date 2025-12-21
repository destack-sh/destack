# formatter

Code formatter for Destack, also supports TypeScript and JavaScript.
Formats `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx` files using the FIR document model.
The primary goal is `.ds` files.
We don't expect anyone to drop Prettier/Biome for their existing `.ts`/`.js` files.
See the formatter specification tests in [`test/fixtures/formatter/`](../../test/fixtures/formatter/) for examples of specific formatting rules.

## Objectives

The formatter aims to produce output that feels natural for Prettier/Biome users while integrating Destack's additional syntax.
Most formatting decisions match Prettier's behavior: trailing commas, bracket spacing, argument wrapping, etc.
Perfect 100% drop-in compatibility is not a goal, though we do match behavior almost everywhere.

We also support many of Prettier's formatting options (see [Configuration](#configuration)).
Destack-specific constructs (tuples, match expressions, struct literals, ownership modifiers) are formatted to feel like natural extensions of the existing style.

## How It Works

1. **Parse** source into AST
2. **Walk** the AST, emitting FIR document nodes
3. **Print** the document to a string

Each AST node type has formatting rules in `format/`.
The rules build up a FIR document describing the layout.
The printer handles line breaking.

```ds
extension for Expression {
    formatNode(nodeId: LocalNodeId<Expression>, f: Formatter): FormatResult<void> {
        match (this) {
            Binary { left, operator, right } => {
                f.write(left, space, operator, space, right)
            }
            // ... other expression variants
        }
    }
}
```

## Configuration

The formatter supports standard Prettier-like options:

| Option | Default | Description |
|--------|---------|-------------|
| `line_width` | 100 | Target line length |
| `indent_style` | Space | `Space` or `Tab` |
| `indent_width` | 4 | Spaces per indent level |
| `quote_style` | Semantic | `Single`, `Double`, or `Semantic` |
| `trailing_comma` | All | `All`, `Es5`, or `None` |
| `bracket_spacing` | true | Spaces inside `{ }` |
| `arrow_parentheses` | Always | `Always` or `AsNeeded` |
| `bracket_same_line` | false | JSX `>` on same line as last attribute |
| `single_attribute_per_line` | false | Force one JSX attribute per line |

`Semantic` quote style means: double quotes for strings, single quotes for characters.
This matches Destack's distinction between `"string"` and `'c'`.

## Annotations and Comments

Comment and decorator attachment follows the source AST.
The formatter preserves:
- Leading comments before a node
- Trailing comments on the same line
- Decorators in their original positions

Blank lines between statements are normalized: at most one blank line is preserved.

## Import Sorting

When `organize_imports` is enabled, the formatter can sort and group imports:

```typescript
// before
import { z } from "c";
import { a } from "a";
import type { T } from "b";

// after (with natural sort)
import { a } from "a";
import type { T } from "b";
import { z } from "c";
```

Type imports sort before value imports within the same group.
Natural sort handles numbers correctly: `a1`, `a2`, `a10` instead of `a1`, `a10`, `a2`.

## Destack-Specific Formatting

These are formatting rules that are specific to Destack.
We tried to make them feel as natural as possible to TS people.

### Tuples

Tuple expressions and types use parentheses:

```
const point: (int32, int32) = (1, 2);
```

### Patterns

Match arms are formatted with consistent indentation:

```
const result = match (value) {
    Ok(x) => x,
    Err(e) if (e.retryable) => retry(),
    Err(e) => fail(e),
};
```

### Structs

Named struct construction:

```
Point { x: 1, y: 2 }

Point {
    x: some_long_expression,
    y: another_long_expression,
}
```

### Where

Generic constraints format naturally:

```
function merge<T, U>(): T where (
    T: Mergeable,
    U: Comparable<T>,
) {
    // ...
}
```

### Ownership

Ownership annotations stay attached to their types:

```
function process(data: &mut Buffer, out: ^Result): &Output {
    // ...
}
```