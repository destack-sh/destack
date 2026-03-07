# PARSEAR plan

This plan is intentionally destructive.
The goal is not parser beauty, Oxc parity, or incremental cleanup.
The goal is `<=100 ms` total parse time on `language/test/fixtures/parser/oxc/typescript.js`, while keeping the wider JavaScript-family parser viable for JS, TS, JSX, TSX, and later `.ds` reuse.

## Current measured state

The relevant working baseline is the current legacy parser, not `ParserV2`.

Measured on `typescript.js`:

- `parse/single-thread`: about `160-166 ms`
- practical recent best: about `159.9-161.4 ms`

Measured hot buckets from `print_single_file_timings`:

- `parse.expression`: about `56 ms`
- `parse.lex.matching_pair`: about `52 ms`
- `parse.expression.primary.identifier`: about `36 ms`
- `parse.alloc.identifier_intern`: about `31 ms`
- `parse.expression.infix`: about `29 ms`
- `parse.expression.postfix`: about `28 ms`
- `parse.alloc.node`: about `25 ms`
- `parse.block.body`: about `22 ms`
- `parse.lex.line_terminator`: about `15 ms`
- `parse.path`: about `15 ms`
- `parse.lex.keyword`: about `14 ms`

Important negative results:

- `parse.alloc.mark`: tiny
- `parse.alloc.restore`: tiny
- the old `ParserV2` experiment proved parser-shape cleanup alone is not enough
- plain parenthesized-group follow scanning regressed badly and should stay reverted
- the single-or-many `LocalStringPool` bucket experiment regressed catastrophically and should stay reverted

Implemented wins already in this diff:

- removed forced full-file prelex from the main parse path
- removed eager identifier-escape caching from `TokenStream`
- made keyword caching lazy
- replaced some type-literal lookahead interning with raw source-text checks
- made `NodeSourceMap` side spans sparse on write
- aggressively pre-sized parser, token-stream, and source-map hot buffers
- replaced the plain parenthesized lambda fast path with a bounded structural scan

## Attack order

The work order should follow measured pressure, not architectural preference.

1. `language/parser/src/lex/stream.rs`: remove token-stream metadata tax
2. `language/ast/src/tree/tree.rs` and `language/source/src/tree/map.rs`: reduce node insertion cost
3. `language/base/src/string.rs` and parser identifier lookup paths: reduce string interning cost
4. `language/parser/src/parse/*`: trim remaining parser service churn after the shared front-end cost is lower

## Track 1: TokenStream teardown

The current `TokenStream` is still a metadata warehouse.
It owns:

- `next_non_newline`
- `matching_pairs`
- `token_keywords`
- `line_terminators_before`
- `leading_comment_before`
- side-trivia ownership ranges
- delimiter stacks
- split-token state

That is too much per-token work.

### 1A. Kill eager metadata

Status:

- forced full-file prelex removed
- eager identifier-escape caching removed
- eager keyword classification removed

Next:

- remove `leading_comment_before` if the parser does not need it in hot paths
- remove or lazify side-trivia ownership vectors from the hot lex path
- isolate comment and blank trivia ownership into the attach-trivia phase only

### 1B. Kill matching-pair tax

Target files:

- [stream.rs](/Users/florian/symbol/destack-6/language/parser/src/lex/stream.rs)
- [parser.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/parser.rs)
- consumers in:
  - [expression/lookahead.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/expression/lookahead.rs)
  - [function.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/function.rs)
  - [type.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/type.rs)

Current problem:

- every delimiter token participates in global stack maintenance
- `matching_pair_or_lex` is still a major cost center

Attack options:

- move delimiter matching to a lighter parser-owned on-demand service
- stop maintaining `matching_pairs` for every token during ordinary lexing
- only cache pair results for openings actually queried by the parser

Rule:

- do not add hints or special lanes
- delete global maintenance work if on-demand work is cheaper overall

### 1C. Kill line-break query churn

Target files:

- [stream.rs](/Users/florian/symbol/destack-6/language/parser/src/lex/stream.rs)
- [parser.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/parser.rs)
- [expression/continuation.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/expression/continuation.rs)

Current problem:

- `line_terminator_before` is still hit extremely often
- parser code often asks the stream repeatedly instead of carrying the answer forward

Attack:

- collapse repeated line-break checks into cursor results where possible
- prefer passing `has_line_break_before` through parser control flow instead of requerying
- remove line-break metadata that is only used by cold paths

### 1D. Reconsider full token storage

This is the brutal fallback if 1A through 1C do not move enough.

Target:

- stop treating parse as “index into `Vec<TokenSpan>` plus services”
- let parser own a narrower current-token and fixed-lookahead state
- keep full token storage only for post-parse trivia or diagnostics

This is the closest thing to an Oxc-class front-end cut inside the current parser.

## Track 2: NodeTree insertion cost

Target files:

- [tree.rs](/Users/florian/symbol/destack-6/language/ast/src/tree/tree.rs)
- [map.rs](/Users/florian/symbol/destack-6/language/source/src/tree/map.rs)
- [arena.rs](/Users/florian/symbol/destack-6/language/base/src/arena.rs)

Current problem:

- every node insert updates several structures
- `NodeSourceMap::append` pushes into multiple vectors and invalidates position indexing

### 2A. Slim the write path

Attack:

- reduce `NodeSourceMap::append` to the minimum required hot data
- defer `main/type/side` span structures until needed
- keep the interval-tree invalidation off the hot path unless a position index already exists

### 2B. Reconsider typed-arena fanout

Attack:

- measure whether `node_type_by_node_id` plus `local_id_by_node_id` can be compressed or deferred
- consider a flatter storage layout for the hot parse path, then lower to full `NodeTree` later if needed

Rule:

- do not build a parallel AST unless it clearly beats incremental slimming

## Track 3: Identifier and string cost

Target files:

- [string.rs](/Users/florian/symbol/destack-6/language/base/src/string.rs)
- [parser.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/parser.rs)
- [key.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/key.rs)
- [type.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/type.rs)
- [expression/expression.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/expression/expression.rs)

Current problem:

- identifier interning alone is still a large bucket
- many parser decisions only need source-text equality, not a stable `StringId`

Attack:

- replace lookahead uses of `identifier_for_index` with raw text comparison where possible
- keep interning only for AST-producing paths
- consider `intern_no_dedupe` for known high-throughput, low-reuse paths if the measurement justifies it

Rule:

- no speculative micro-fast-lanes
- remove unnecessary interning from parser logic first

## Track 4: Parser service cleanup

Target files:

- [parser.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/parser.rs)
- [expression/continuation.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/expression/continuation.rs)
- [expression/expression.rs](/Users/florian/symbol/destack-6/language/parser/src/parse/expression/expression.rs)

This track comes after the shared front-end cost is lower.

Attack:

- pass cursor facts through instead of requerying stream services
- remove parser helpers that only exist to service the old token-warehouse architecture
- only then revisit expression and statement control flow

## Validation loop

After each structural cut:

1. `cargo +nightly-2025-11-27 fmt --all`
2. `cargo +nightly-2025-11-27 check -p destack_parser`
3. `cargo +nightly-2025-11-27 test -p destack_parser --quiet`
4. `DESTACK_PARSER_TIMINGS=1 cargo +nightly-2025-11-27 run -p destack_parser --release --example print_single_file_timings --features parser_timings -- language/test/fixtures/parser/oxc/typescript.js`
5. `DESTACK_PARSE_FILE=language/test/fixtures/parser/oxc/typescript.js cargo +nightly-2025-11-27 bench -p destack_parser --bench destack_parse -- 'parse/single-thread'`

## Immediate next cuts

These are the next sync points in order:

1. rip out more `matching_pairs` global maintenance
2. reduce repeated line-break service calls in expression and block parsing
3. remove unnecessary `identifier_for_index` use from lookahead paths
4. slim `NodeTree::insert` and `NodeSourceMap::append`
