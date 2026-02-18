# Formatter Architecture Review

## Scope

This review covers `language/formatter/src/format`.
The goal is a final ownership model that is explicit, composable, and fast.
The target is oxfmt and prettier conformance with predictable performance behavior.

## Method

I used the local formatter inventory script at `language/formatter/scripts/inventory_symbols.sh`.
I reviewed module boundaries, `FormatNode` implementation locations, and cross-domain imports.
I compared this shape against `~/symbol/oxc/crates/oxc_formatter`, especially `print/*`, `utils/*`, and `formatter/*`.

## Current Snapshot

The formatter currently has 102 Rust files and 37,330 lines.
It currently has 927 functions, with 848 free functions and 79 methods.
The largest files are `language/formatter/src/format/tree.rs` at 2,039 lines and `language/formatter/src/format/expression/operator/binary.rs` at 1,558 lines.
The largest function-heavy files are `language/formatter/src/format/annotation/node.rs`, `language/formatter/src/format/tree.rs`, `language/formatter/src/format/expression/parentheses.rs`, and `language/formatter/src/format/chain/classify.rs`.

## Current Domain Map

`analysis`: call layout facts, scan helpers, and timing tags.
`annotation`: rendering and `FormatNode` impls for comments, docs, blanks, and decorators.
`call`: argument nodes plus call layout and call rendering.
`chain`: member chain normalization, break analysis, and render policy.
`collection`: key, literal, path, pattern, property formatting, and shared collection scoring.
`comments`: trivia projection, seam ownership, and comment placement internals.
`context`: formatter context, caches, source helpers, annotation helpers, and node dispatch trait.
`declaration`: declaration and statement-level formatting plus dependency and signature formatting.
`directive`: ignore directives and directive scanning logic.
`expression`: expression dispatch plus operator and precedence-sensitive formatting.
`tree`: TSX and tree literal formatting and hugging rules.

## `FormatNode` Ownership Map

`Declaration`, `Block`, `MatchCase`, `DependencyItem`, `WhereClause`, and `EnumField` live under `declaration`.
`Expression` and `Declarator` live under `expression`.
`Pattern`, `PatternField`, `Property`, and `Member` live under `collection`.
`Annotation`, `Comment`, `Doc`, `Decorator`, and `Blank` live under `annotation`.
`Argument` lives under `call/argument.rs`.
`Parameter` lives under `declaration/signature.rs`.

## Coupling Findings

The main couplings are `expression -> analysis`, `declaration -> call`, `call -> analysis`, and `analysis -> expression`.
`chain` and `tree` still depend on `crate::expression::*` broad imports.
`call` no longer depends on a broad expression prelude through `call/mod.rs`.
`expression/mod.rs` is still acting as a large shared prelude for non-expression domains.
This makes domain ownership harder to reason about and increases refactor risk.

## Structural Issues To Fix

Large files mix multiple concerns, especially `tree.rs`, `declaration/statement.rs`, and `expression/operator/binary.rs`.
Cross-domain visibility is currently managed by broad wildcard imports instead of explicit domain APIs.
README architecture guidance is stale and still refers to removed files and old ownership framing.

## OXC Comparison

OXC keeps printing concerns under `print/*` and reusable heuristics under `utils/*`.
OXC keeps document engine and context concerns under `formatter/*`.
OXC isolates member chain logic in `utils/member_chain/*` and call argument layout in `print/call_like_expression/arguments.rs`.
OXC still has some large files, but ownership boundaries are explicit and stable.
Destack should keep its chosen top-level domains while adopting OXC-like explicit ownership boundaries and utility placement.

## Gold Standard Target

Top-level formatter domains should remain `analysis`, `annotation`, `call`, `chain`, `collection`, `declaration`, `directive`, `expression`, and `tree`.
`context` and `comments` should remain internal engine layers.
Each domain should only expose a small explicit API surface and avoid wildcard cross-domain imports.
Cross-domain reusable helpers should live in explicit shared modules instead of being owned by syntax-specific domains.
`FormatNode` implementations should live in the domain that owns the semantic node kind.

## Required Final Moves

Replace wildcard cross-domain imports with explicit imports and explicit module exports.
Split the largest multi-concern files into single-purpose files with single-part names.
Keep `comments/*` as the only place for comment ownership policy and seam fallback rules.
Update README architecture text to match the final structure and ownership model.

## Concrete Destination Layout

`analysis/*`: call shape analysis, expansion profiles, layout facts, scan helpers, and timing tags.
`call/*`: call printing and call-argument rendering only.
`chain/*`: chain normalization, chain break policy, and chain rendering only.
`declaration/*`: declaration and statement syntax printing and parameter ownership.
`collection/*`: reusable list and collection rendering primitives plus pattern and property ownership.
`expression/*`: expression-only precedence and syntax dispatch.
`tree/*` or `tree.rs`: tree and TSX formatting only, with explicit imports from shared and expression APIs.

## Migration Strategy

First, carve explicit public crate-local APIs in each domain module.
Second, replace wildcard imports and remove transitional re-export glue.
Third, split large files where they still mix unrelated policy clusters.

## Validation Gates

Run `cargo check -p destack_formatter` after each domain move.
Run `cargo test --release -p destack_formatter` after each ownership batch.
Run `cargo test --release -p destack_test --test formatter` at each behavior checkpoint.
Use existing `bench_stats` runs to ensure structural changes do not regress throughput.
