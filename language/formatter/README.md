# formatter

Code formatter for Destack, also supports TypeScript and JavaScript.
Formats `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx` files using the FIR document model.
The primary goal is `.ds` files.
We don't expect anyone to drop Prettier/Biome for their existing `.ts`/`.js` files.
See the formatter specification tests in [`test/fixtures/formatter/`](../test/fixtures/formatter/) for examples of specific formatting rules.

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

## Module Layout Rules

Formatter modules are grouped by domain ownership first.
Top-level domains are `analysis`, `annotation`, `call`, `chain`, `collection`, `declaration`, `directive`, `expression`, and `tree`.
Engine internals are under `context` and `comments`.
Cross-domain helpers should live in neutral owners and not under syntax-specific domains.
Node formatting impls should live with semantic owners, not convenience owners.
Wildcard cross-domain imports are discouraged because they hide ownership and increase coupling.

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

## Benchmarks

Formatter performance is measured with the same `bench_stats` pattern used by other language crates.
Run `cargo run --release -p destack_formatter --example bench_stats -- --help` for all options.
Default corpus root is `test/fixtures/ecosystem/checkouts`.
Default corpus profile is `standard`, which targets a larger representative workload for iterative perf work.
Use `--corpus full` to benchmark all fetched ecosystem checkouts under `--root`.
Use `--corpus quick` for a smaller representative set.
For quick smoke runs you can use `--root test/fixtures/formatter/conformance/staging/oxfmt`.
Use `--mode real-world` to respect file-level ignore directives and report product behavior.
Use `--mode engine` to disable file-level ignore directives for cross-tool fairness.
Benchmark output reports both total corpus throughput and formatted-only throughput.
Use `--output table|csv|json` for human-readable output or machine-readable pipelines.
The table output includes colorized latency heat, stage breakdowns, throughput, jitter, and hot-file rankings.
Use `--timings` to enable internal formatter instrumentation.
Timing output includes top timing tags and cache hit rates for span and annotation lookups.

## Destack-Specific Formatting

These are formatting rules that are specific to Destack.
We tried to make them feel as natural as possible to TS people.

### Tuples

Tuple expressions and types use parentheses:

```ds
const point: (int32, int32) = (1, 2);
```

### Patterns

Match arms are formatted with consistent indentation:

```ds
const result = match (value) {
    Ok(x) => x,
    Err(e) if (e.retryable) => retry(),
    Err(e) => fail(e),
};
```

### Structs

Named struct construction:

```ds
Point { x: 1, y: 2 }

Point {
    x: some_long_expression,
    y: another_long_expression,
}
```

### Where

Generic constraints format naturally:

```ds
function merge<T, U>(): T where (
    T: Mergeable,
    U: Comparable<T>,
) {
    // ...
}
```

### Ownership

Ownership annotations stay attached to their types:

```ds
function process(data: &readonly Buffer, out: ^Result): &readonly Output {
    // ...
}
```

## Testing

Run these from the repository root.

### Quick local loop

```sh
cargo test -p destack_formatter
just language/test-formatter
```

### Shared test gates

```sh
just language/quick
just language/full
```

### Conformance coverage

```sh
just language/install-formatter-conformance
just language/test-formatter-conformance
```

### Performance and fuzzing

```sh
just language/benchmark-formatter-stats "--help"
just language/fuzz-formatter 300
```
