# formatter

Code formatter for Destack, TypeScript, JavaScript, JSX, and TSX.
The correctness target for the shared JS and TS surface is `oxc_formatter`.

Unsupported legacy or non-strict syntax belongs in conformance status files instead of formatter heuristics.
Destack-only syntax should fit into the same structural model rather than inventing separate policy when an OXC-shaped analogue exists.

## Goals

The formatter should be structurally close to OXC on every supported JS and TS family.
Parser, trivia, FIR, and formatter support layers should be reshaped when the current model blocks an exact OXC port.
Point fixes are a last step, not the main strategy.

For Destack-only syntax, the target is OXC-style discipline rather than literal OXC parity.
The output should still feel native next to the JS and TS formatting rules it extends.

## Architecture

Formatting is a three-stage pipeline:

1. Parse source into the Destack AST plus raw trivia.
2. Walk the AST and emit FIR nodes.
3. Print FIR to text with width-aware grouping and line breaking.

The formatter uses the raw-comment cursor model instead of pre-attached semantic comment ownership.
That keeps comment handling closer to OXC and avoids a second attachment layer in the parser.

## Current Strategy

The active migration strategy is literal family-by-family porting from `~/symbol/oxc` wherever Destack supports the same syntax.
If a local helper does not have a convincing upstream peer, it should be treated as suspicious.
If a port feels awkward because of local IR or trivia shape, the support layer should be simplified until the port becomes direct.

The current audit artifacts are:

- [FORMATTER_MAP.md](/Users/florian/symbol/destack-6/FORMATTER_MAP.md)
- [FORMATTER_PLAN.md](/Users/florian/symbol/destack-6/FORMATTER_PLAN.md)
- [HANDOFF.md](/Users/florian/symbol/destack-6/HANDOFF.md)

## Configuration

The formatter exposes the standard workspace formatting controls for width, indentation, commas, quote style, bracket spacing, and related layout policy.
Those options should influence formatting in the same places OXC already allows, rather than creating new local branches.

## Testing

Run these commands from the repository root.

```sh
cargo check -p destack_formatter
cargo test -p destack_formatter
cargo test -p destack_test --test formatter
cargo test -p destack_test --test conformance-formatter
just fmt
```
