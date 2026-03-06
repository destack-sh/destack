# Performance Plan

This plan covers parser and formatter throughput work for Destack.
This snapshot is current as of March 6, 2026.

## Scope

The goal is substantial parser and formatter speedups without losing correctness, conformance, or maintainability.
The primary comparison target is Oxc on the same machine for overlapping JS, TS, and TSX workloads.
Destack syntax is more expressive than standard TS, so exact parity is not required on every construct.
However, standard TS and TSX workloads should be much closer than they are today.
The working parser milestone for this file is now `~50 ms` parse-only on `language/test/fixtures/parser/oxc/typescript.js`, which is the practical interpretation of “within roughly 2x of Oxc” on the current machine.
The formatter remains a separate larger wall clock problem, so `50 ms` should currently be treated as a parser target rather than an end-to-end formatting target.

## Current Position

Parser and formatter correctness remain non-negotiable.
The old parser workstream was scoped around formatter correctness and conformance.
That work is now folded into this broader performance workstream instead of living in a separate parser-only plan.

### Current Destack Baseline

Measured on `language/test/fixtures/parser/oxc/typescript.js` with `cargo run --release -p destack_formatter --example bench_stats -- --root language/test/fixtures/parser/oxc/typescript.js --corpus full --runs 3 --warmup-runs 1 --workers 1 --timings --top 5 --output json --no-progress`.
Source size is `8,207,497` bytes and `171,677` lines.
End-to-end formatter work on that file is about `1979.8 ms`.
Parser total is about `188.9 ms`.
Parser main is about `152.4 ms`.
Parser trivia attach is about `17.7 ms`.
Parser post-work outside parse-main and trivia attach is about `18.8 ms`.
Formatter document building is about `1306.7 ms`.
Printer cost is about `53.4 ms`.

### Current Measured Progress

The first parser stack-guard simplification reduced the same benchmark from about `1979.8 ms` total work to about `1933.3 ms`.
That change reduced parser total from about `188.9 ms` to about `182.7 ms`.
That change reduced parser main from about `152.4 ms` to about `149.0 ms`.
That change reduced formatter document building from about `1306.7 ms` to about `1271.4 ms`.

The borrowed formatter annotation iteration pass was structurally good but not yet a breakthrough.
On a lower-noise rerun before the machine became heavily contended, formatter document building was about `1269.7 ms`.
That is directionally better than `1271.4 ms`, but still too small to count as a real win by itself.

Parser Criterion no-drop timing on the same file later measured about `176.7 ms` to `178.4 ms`.
Criterion compared that to an older local base and flagged a `+2.4%` to `+4.4%` regression, so that signal needs a clean rerun after contention clears.

The current trusted parser-only `bench_stats --parse-only` mean on the same file is now about `176.3 ms`.
That is down from the earlier parse-only baseline around `184.2 ms`.
Parser main is now about `150.8 ms`.
Parser trivia attach is now about `18.2 ms`.
This is a real parser win, but it is still far from the Oxc target band.

Very recent end-to-end benchmark runs that landed in the `2.8 s` to `3.3 s` band are not trusted.
At the time of those runs, system load average was about `14.93`, `14.34`, and `13.10`.
Treat those results as invalid for decision making.

### Initial Hotspot Read

Parser main is still the dominant parser cost.
Trivia attachment is large enough to matter and should not be treated as noise.
Formatter annotation and expression-heavy formatting dominate the current end-to-end runtime far more than parse.
Current formatter instrumentation shows very high annotation query volume and many cache misses on hot call and expression paths.

### Systematic Gap Versus Oxc

The remaining parser gap is structural, not a single isolated bug.
Destack still performs more work per token than Oxc on overlapping TS input.
The hot path repeatedly normalizes scanner state instead of mostly advancing one current token through tight local loops.
The parser still swaps broad `ParserOptions` state more often than a hot expression engine should.
Parenthesized heads and lambda disambiguation still pay repeated follow checks, lookahead probes, and fallback paths.
AST bookkeeping still does more per-node work than a tighter parser would ideally do in its hot path.
Trivia ownership is still reconstructed after parse instead of being emitted incrementally during lexing or parse.
This means point fixes can keep buying modest wins, but the remaining gap will only close materially with structural simplification of the parser core.

## Success Criteria

We will track success with exact benchmark numbers, not vibes.
The parser target is to cut `typescript.js` parse total dramatically from the current `~189 ms` into a much smaller band that is materially closer to Oxc.
The formatter target is to cut the same file end-to-end formatting time dramatically from the current `~1980 ms`.
Every significant change should either improve measured throughput or simplify the code enough to unlock the next optimization tranche.

### Target Bands

The immediate parser milestone is `<= 50 ms` parse-only on `typescript.js`.
An intermediate parser milestone is `<= 90 ms`, which would already represent a substantial structural win from the current `~176 ms`.
The immediate formatter milestone is not `50 ms`, because current formatter wall time is far larger and needs its own later structural pass.
End-to-end formatting should still be tracked on every tranche so parser wins are not masked by formatter regressions.

## Non Goals

We will not trade away parser or formatter correctness for benchmark wins.
We will not move semantic validation out of later compiler stages just to avoid parser work.
We will not preserve slow accidental architecture if a simpler and more idiomatic design is clearly better.
We will not chase unsupported proposal syntax just because another tool parses it.

## Parser Versus Formatter Boundary

Keep parse-level syntax constraints in the parser.
Keep semantic validity rules in compiler analyze and validate.
Use parser changes when parser structure or parser-emitted facts are the real bottleneck.
Use formatter changes when formatting cost comes from annotation lookup, source scanning, tree analysis, or document construction on already-correct AST and token facts.

## Measurement Protocol

Always compare warm runs to warm runs.
If the machine looks contended or the numbers wobble abnormally, wait and rerun.
Prefer single-file targeted benchmarks for hotspot work and corpus benchmarks for regression checks.
Record exact commands and results for every meaningful optimization tranche.

### Primary Commands

Parser single-file baseline:
`cargo run --release -p destack_formatter --example bench_stats -- --root language/test/fixtures/parser/oxc/typescript.js --corpus full --runs 3 --warmup-runs 1 --workers 1 --timings --top 5 --output json --no-progress`

Parser-only single-file baseline through the same harness:
`cargo run --release -p destack_formatter --example bench_stats -- --root language/test/fixtures/parser/oxc/typescript.js --corpus full --runs 3 --warmup-runs 1 --workers 1 --parse-only --output json --no-progress`

Parser-main-only single-file baseline through the same harness:
`cargo run --release -p destack_formatter --example bench_stats -- --root language/test/fixtures/parser/oxc/typescript.js --corpus full --runs 3 --warmup-runs 1 --workers 1 --parse-only --no-trivia --output json --no-progress`

Parser criterion bench:
`DESTACK_PARSE_FILE=/abs/path/to/file cargo bench -p destack_parser --bench destack_parse`

Parser flamegraph:
`language/parser/scripts/profile_single_file.sh language/test/fixtures/parser/oxc/typescript.js parse`

Formatter corpus bench:
`cargo run --release -p destack_formatter --example bench_stats -- --root test/fixtures/ecosystem/checkouts --corpus standard --runs 3 --warmup-runs 1 --workers 1 --timings --top 20 --output table`

## Validation Gates

Every parser or formatter change in this workstream must pass the relevant gates before we call it done.

### Core Correctness Gates

- `cargo test -p destack_parser --quiet`
- `cargo test --release -p destack_test --test parser-conformance --quiet`
- `cargo test -p destack_formatter --quiet`
- `cargo test --release -p destack_test --test formatter --quiet`
- `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`

### Performance Regression Checks

- Rerun the single-file `typescript.js` benchmark after each optimization tranche.
- Rerun the formatter standard corpus benchmark after any formatter cache or annotation-layout rewrite.
- Recheck Oxc comparison points after major parser refactors to ensure we are actually closing the gap.

## Prioritized Workstreams

## 1. Parser Core

Focus on the hottest expression and continuation paths first.
Reduce speculative parsing, option swapping, and repeated token classification in the main expression loop.
Push more work into cached token facts and cheap plain paths when the grammar shape is already obvious.
Prefer parser-local borrowed facts over repeated string or AST lookups when staying on the hot path.
Bias toward deleting machinery and merging overlapping helper layers when the same grammar fact is rediscovered more than once.

### Parser Hypotheses

- Expression continuation is doing too much branching and repeated state normalization per token.
- Parser option swapping and speculative mark or restore paths are still too frequent.
- Delimiter lookahead and newline-aware scanner cursor work are probably paying too much per grouped expression.
- Path, static-argument, and call continuation handling likely redoes facts that can be cached once per token.

### Parser Structural Rewrite Agenda

1. Rewrite the hot expression core around tighter token-directed dispatch.
2. Rewrite parenthesized and arrow disambiguation as one explicit subsystem with one fast path and one fallback.
3. Collapse hot-path parser context handling so ordinary expression edges stop carrying the full `ParserOptions` machinery.
4. Push trivia ownership closer to lexing or parse-time emission instead of rebuilding it after parse.
5. Reduce AST hot-path bookkeeping where spans or side tables can be filled more directly.

## 2. Parser Trivia And Annotation Attachment

Keep comment and blank ownership correct, but make attachment cheaper.
Avoid extra whole-stream passes when ownership facts can be emitted incrementally during lexing or parse.
Avoid repeated span string extraction and repeated seam newline checks on the same trivia.
Study Oxc and Biome approaches for attaching or storing trivia during lexing rather than rebuilding ownership later.

### Parser Trivia Hypotheses

- `attach_trivia` still pays too much for full-stream materialization and post-hoc indexing.
- Documentation and comment ownership indexes may be recomputing facts that are already implicit in lexed token adjacency.
- Trivia-only files and idempotence safeguards are correct but should have a cheaper fast path.

## 3. Formatter Annotation Hot Paths

Reduce annotation query overhead before deeper layout rewrites.
Prefer borrowed annotation iteration over cloned `Vec`s in hot formatter code.
Precompute dense node-local annotation flags and small hot summaries once when building formatter context.
Push hot call-layout and expression-layout predicates onto dense caches keyed by node id.

### Formatter Hypotheses

- Returning cloned annotation vectors is too expensive on hot paths.
- `has_annotation` style predicates are good, but many follow-up paths still fall back to repeated full annotation scans.
- Source token scanning helpers in formatting decisions are likely re-walking the same spans too often.

## 4. Formatter Expression And Call Layout

After annotation data access is cheaper, rework the most expensive expression and call-layout paths.
Use measured timing tags and counters to drive this work, especially `format.expression.statement`, `format.expression`, `format.expression.operator`, `format.expression.call`, and call argument layout paths.
Prefer one analysis pass that feeds multiple layout decisions over many tiny repeated scans.

## 5. Structural Simplification

When a hot path is messy, simplify it instead of micro-optimizing noise around it.
Prefer simpler control flow and denser data over helper layers that exist only for abstraction.
Delete dead compatibility shims and stale plan assumptions as soon as the replacement is stable.

## Immediate Execution Order

1. Prepare the current green simplification and speedup tranche for check-in without mixing it with the next rewrite.
2. Extend the unified `bench_stats` harness to surface parser control-flow counters in the same output path instead of relying on separate bench-only printing.
3. Measure grouped-head and lambda disambiguation with those counters on `typescript.js`.
4. Rewrite parenthesized and arrow disambiguation to stop repeating follow and shape work across expression and function parsing.
5. Remeasure parser single-file performance and validate parser tests.
6. Start the trivia ownership redesign once the grouped-head rewrite settles.
7. Return to formatter structural work after parser reaches a much lower band.

## Work Log

### March 6, 2026

- Replaced the old parser-only plan with this combined performance plan.
- Confirmed that the old `PARSER_PLAN.md` was correctness-scoped and explicitly excluded performance-only rewrites.
- Measured Destack baseline on `language/test/fixtures/parser/oxc/typescript.js`.
- Identified parser main, trivia attachment, and formatter annotation-heavy expression paths as the current frontiers.
- Began local Oxc benchmark verification on the same machine.
- Generated a parser flamegraph for `language/test/fixtures/parser/oxc/typescript.js` with `CARGO_TARGET_DIR=/tmp/destack-target DESTACK_PARSE_FILE=/Users/florian/symbol/destack-6/language/test/fixtures/parser/oxc/typescript.js cargo bench -p destack_parser --bench destack_parse -- parse/no-drop --profile-time 3`.
- The parser flamegraph pointed at `Parser::parse_root_expressions`, `Parser::eat_block_body`, `Parser::try_dispatch_identifier_statement_expression`, `Parser::eat_statement_expression_from_token_kind`, `Parser::try_eat_plain_parenthesized_lambda`, and `Parser::parse_plain_parenthesized_expression_unchecked`.
- Added a periodic statement stack growth guard so statement parsing no longer pays `ensure_sufficient_stack` on every recursive step.
- Reserved the local string pool index map with `FxHashMap::with_capacity_and_hasher` in `language/base/src/string.rs`.
- Added borrowed formatter annotation helpers with `visit_annotations`, `any_annotation_id`, and `find_annotation_id` consumers across call-argument, tree-argument, and binary-operator hot paths.
- Measured the borrowed formatter annotation pass as structurally cleaner but only marginally faster on the trusted rerun.
- Simplified parser identifier statement dispatch by reusing a dedicated plain-identifier fast path instead of re-entering the general identifier probe after a keyword miss.
- Simplified parser function parsing by hoisting plain-lambda eligibility checks into `eat_function_inner` and avoiding redundant plain-lambda probe guards.
- Extended `bench_stats` so the same harness now supports parser-only stage selection with `--parse-only`, `--no-trivia`, `--no-format`, and `--no-print`.
- Confirmed that unified parser-only bench output stays in the same JSON and table shape, with disabled phases reported as zero instead of requiring a separate benchmark binary.
- Rewrote the parser postfix continuation loop into token-directed dispatch so hot postfix steps no longer precompute unrelated lookahead and control-flow state on every iteration.
- Tightened parser stack growth checks in debug builds to keep deep recursive parser tests green after the postfix refactor, while leaving release builds on a looser interval.
- Fixed optional chaining and indirect postfix regressions from that refactor, including `?.[` chains, `.?` tails, and optional chaining after comment-separated newlines.
- Remeasured the unified parse-only harness on `language/test/fixtures/parser/oxc/typescript.js` at about `176.3 ms` mean parse plus trivia, down from about `184.2 ms`.
- Revalidated the parser and formatter test suites with `cargo test -p destack_parser` and `cargo test -p destack_formatter`.
- Reconfirmed that the benchmark machine can become heavily contended and that outlier runs must be discarded instead of rationalized.
- Audited the remaining parser gap directly against Oxc control flow and confirmed that the largest remaining costs are structural: scanner normalization, broad context swapping, grouped-head reanalysis, post-parse trivia ownership, and heavier AST bookkeeping.
- Set the explicit next parser milestone to `~50 ms` parse-only on `typescript.js`, with the understanding that end-to-end formatter wall time is a separate later problem.
- Froze the current workstream boundary for check-in: keep the existing parser continuation, statement dispatch, lambda plain-path, parse-only harness, string-pool, and formatter annotation wins together, and defer parser instrumentation plus grouped-head rewrite to the next tranche.

## Check In Boundary

The current check-in boundary is the already-landed simplification and speedup work that remains green.
That includes the parser statement-dispatch simplification, the parser plain identifier and plain lambda gating cleanup, the postfix continuation rewrite, the debug-sensitive stack guard adjustment, the unified parse-only `bench_stats` path, the string-pool reservation improvement, and the formatter borrowed annotation helpers.
The next tranche should begin with parser instrumentation in the unified harness and the grouped-head rewrite.
Do not mix those larger structural changes into the current check-in unless they are already fully benchmarked and revalidated.

## Successor Notes

Keep updating this file with exact commands, exact timings, and exact next steps.
If a benchmark result looks suspicious, rerun it after contention clears instead of rationalizing it.
If a change is faster but uglier, either simplify it immediately or leave a concrete follow-up note here.
Prefer one benchmark harness and one output format, even when adding parser-only counters or memory instrumentation.
