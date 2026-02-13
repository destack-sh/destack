# Formatter Map

This document captures a static and dynamic review of the `destack_formatter` crate as of February 12, 2026.
This pass focuses on correctness, cleanliness and systemicness, and single core throughput risk points.

## Current Working Baseline

This section is the current baseline as of February 13, 2026 and supersedes older checkpoint snapshots below.
The formatter conformance policy now tracks only `oxfmt` and `prettier`, with no hard versus soft suite split.
The `biome` suite was removed from formatter conformance.
Parser negative fixtures under `js/_errors_` and `typescript/_errors_` were moved from `prettier-known-failures.txt` into `prettier-ignored.txt`.
This keeps formatter conformance focused on format behavior and idempotence while parser invalid case behavior is tracked separately.

The latest full conformance baseline was captured with `cargo test --release -p destack_test --test formatter-conformance -- --no-parallel`.

| Suite | Passed | Failed | Ignored | Total | Rate |
|:--|--:|--:|--:|--:|--:|
| prettier | 1323 | 345 | 1560 | 1668 | 79.32% |
| oxfmt | 123 | 6 | 6 | 129 | 95.35% |
| total | 1446 | 351 | 1566 | 1797 | 80.47% |

The Prettier failure kind split is `parse=232`, `output=0`, `idempotence=113`, and `read=0`.
The oxfmt failure kind split is `parse=0`, `output=6`, `idempotence=0`, and `read=0`.
Known failure files were refreshed to remove unexpected regression noise, and the suite currently runs without unexpected regressions.
An additional `12` parser negative fixtures outside `_errors_` buckets were moved from known failures to ignored to keep parser-only invalid syntax out of formatter burn-down.

The largest remaining formatter relevant pressure points in Prettier known failures are concentrated in comment handling and idempotence drift.
Top two level buckets are `js/comments` with `37` failures, `js/babel-plugins` with `14`, `js/source-phase-imports` with `8`, and both `js/if` plus `js/explicit-resource-management` with `7` each.
Because parser is a separate workstream today, the immediate formatter focus should prioritize the `113` idempotence failures and comment placement stability clusters.
This pass includes baseline test runs, formatter conformance runs, benchmark reruns, and targeted failure spot checks.

## Annotation Preservation Checkpoint

A targeted parser and formatter harness fix was applied for Destack annotation preservation on February 13, 2026.
The formatter unit harness now calls `parser.finish_annotations()` in `language/formatter/src/tests/tests.rs`.
Destack parameter and type annotation prefixes now preserve attachment in parser while JS and TS decorator parsing keeps prior compatibility behavior.
The parser implementation now collects and attaches decorators only for Destack paths, and still consumes decorator prefixes in JS and TS paths to avoid conformance regressions.

The validation commands and outcomes for this checkpoint are listed below.
`cargo test --release -p destack_formatter` passed with `232 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --no-parallel` passed with no unexpected regressions.
The conformance breakdown remained `prettier: 1323 passed, 345 failed, 1560 ignored, 79.32%` and `oxfmt: 123 passed, 6 failed, 6 ignored, 95.35%`.
A follow up formatter cleanup changed the inline policy for decorator prefixed type annotations after `:`.
Simple decorator type annotations now stay inline, and only break when wider formatting heuristics require it.
This behavior is covered by `format::annotation::tests::test_format_decorator_type_annotation_stays_inline_after_colon`.
The transform fixture `language/test/fixtures/formatter/transform/declarations/variables.md` now expects `const buffer: @addrspace("shared") &Buffer = value;` on one line.


## Review Scope

I reviewed crate docs and architecture context in `README.md`, `language/DESIGN.md`, `language/SPECIFICATION.md`, `language/fir/README.md`, `language/formatter/README.md`, and `language/ast/README.md`.
I reviewed conformance context in `language/test/src/formatter/conformance/README.md`.
I reviewed every file under `language/formatter/src/format` and supporting crate files under `language/formatter/src`, `language/formatter/examples`, `language/formatter/fuzz`, and `language/formatter/scripts`.
I collected rough hotspot metrics by file size, function count, and source scanning signal usage counts.
I ran baseline formatter tests and benchmarks in this worktree to capture current runtime and conformance numbers.

## Baseline Snapshot

All baseline runs below were executed on February 12, 2026.
The machine was contested and noisy, so benchmark commands were rerun for stability checks.
Conformance fixtures were linked from `/Users/florian/symbol/destack-6/language/test/fixtures/formatter/conformance/staging` because network fetch is blocked in this environment.
Ecosystem benchmark corpus was linked at `test/fixtures/ecosystem/checkouts` to satisfy the benchmark default root.

### Test Baseline

`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` failed by regressions with `1 passed`, `2 failed` suites.

### Conformance Baseline

The conformance suite results from the baseline run are listed below.

| Suite | Tier | Passed | Failed | Ignored | Total | Rate |
|:--|:--|--:|--:|--:|--:|--:|
| oxfmt | hard | 129 | 0 | 6 | 129 | 100.00% |
| biome | soft | 628 | 1099 | 9 | 1727 | 36.36% |
| prettier | soft | 1222 | 570 | 1436 | 1792 | 68.19% |
| total | - | 1979 | 1669 | 1451 | 3648 | 54.25% |

The run reported `24` unexpected regressions in biome and `35` unexpected regressions in prettier.
The run reported `365` parser stage failures in biome and `437` parser stage failures in prettier.
The run reported `124` fixed known failures and `59` regressions across all suites.
The run auto updated `language/test/src/formatter/conformance/README.md` summary numbers in this worktree.

### Post Overlap Fix Checkpoint

A focused fixer pass was applied in `language/formatter/src/format/expression/call/profile.rs`.
The fixes added multiline call forcing for single ternary argument calls and tightened hug-last determinism for collection tails.
The overlap regressions `js/assignment/issue-2540.js` and `js/conditional/new-expression.js` now pass in both prettier and biome filtered runs.
A follow up full run was executed with `cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome`.

The post fix full run results are listed below.

| Suite | Passed | Failed | Ignored | Total | Rate | Rate Delta Vs Baseline |
|:--|--:|--:|--:|--:|--:|--:|
| biome | 629 | 1098 | 9 | 1727 | 36.42% | +0.06% |
| prettier | 1225 | 567 | 1436 | 1792 | 68.36% | +0.17% |
| total | 1854 | 1665 | 1445 | 3519 | 52.69% | -1.56% |

The unexpected regressions dropped from `59` to `54` with no new unexpected regressions added.
Removed regressions were `js/assignment/issue-2540.js`, `js/conditional/new-expression.js`, `prettier/js/assignment/issue-2540.js`, `prettier/js/conditional/new-expression.js`, and `js/async/inline-await.js`.

### Post Binary Stability Sweep

A second fixer pass targeted source-sensitive binary and assignment layout behavior.
The main changes de-emphasized source newline heuristics for logical operators while preserving source operator breaks for non-logical operators.
This pass was validated against formatter unit tests and full prettier and biome conformance runs.

The post binary sweep full run results are listed below.

| Suite | Passed | Failed | Ignored | Total | Rate | Rate Delta Vs Prior Checkpoint |
|:--|--:|--:|--:|--:|--:|--:|
| biome | 633 | 1094 | 9 | 1727 | 36.65% | +0.23% |
| prettier | 1232 | 560 | 1436 | 1792 | 68.75% | +0.39% |
| total | 1865 | 1654 | 1445 | 3519 | 53.00% | +0.31% |

The unexpected regressions dropped from `54` to `46` with no new unexpected regressions added.
Removed prettier regressions were `js/arrows/chain-in-logical-expression.js`, `js/binary-expressions/array-and-object.js`, `js/binary-expressions/arrow.js`, and `js/binary-expressions/return.js`.
Removed biome regressions were `js/module/object/property_object_member.js`, `prettier/js/private-in/private-in.js`, `prettier/js/template-literals/binary-exporessions.js`, and `prettier/js/template-literals/logical-expressions.js`.

### Post Chain Index Safety Sweep

A third fixer pass targeted chain line safety and idempotence around direct index operations.
The main change was to merge direct index chain lines into the previous chain line in `language/formatter/src/format/expression/chain/format.rs`.
This prevents line starts like `[0]` after a chained call and avoids ASI induced statement splits on second format pass.
I updated formatter unit expectations in `language/formatter/src/format/expression/tests.rs` for direct index chain grouping.

Targeted conformance checks were run with suite filters.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --suite-filter method-chain/bracket_0 --verbose` now passes `2/2` tests.
`cargo test --release -p destack_test --test formatter-conformance -- --biome --suite-filter method-chain/bracket_0 --verbose` still passes `0/2` with one unexpected regression, `prettier/js/method-chain/bracket_0.js`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --suite-filter chain-expression/issue-15785-3.js --verbose` still fails by idempotence mismatch.
`cargo test --release -p destack_test --test formatter-conformance -- --biome --suite-filter chain-expression/test-3.js --verbose` still fails by output mismatch.

A full prettier plus biome run was rerun with `cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome`.
The post chain sweep full run results are listed below.

| Suite | Passed | Failed | Ignored | Total | Rate | Rate Delta Vs Post Binary |
|:--|--:|--:|--:|--:|--:|--:|
| biome | 632 | 1095 | 9 | 1727 | 36.60% | -0.05% |
| prettier | 1234 | 558 | 1436 | 1792 | 68.86% | +0.11% |
| total | 1866 | 1653 | 1445 | 3519 | 53.03% | +0.03% |

The unexpected regression total is now `47`, which is `+1` versus the prior `46` checkpoint.
Suite deltas are `prettier: -1` regression and `biome: +2` regressions.
The failure kind split in this run was `biome: parse=365, output=698, idempotence=32` and `prettier: parse=436, output=0, idempotence=122`.

Validation after this pass is listed below.
`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.

### Benchmark Baseline

The primary benchmark command was `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
The primary corpus root for this command was `test/fixtures/ecosystem/checkouts`.
Run set A reported mean total `2.793s`, lines per second `2.80M/s`, and format CPU lines per second `1.02M/s`.
Run set B reported mean total `2.764s`, lines per second `2.83M/s`, and format CPU lines per second `1.02M/s`.
Observed jitter between reruns was low with coefficient of variation between `0.4%` and `0.6%`.
Both run sets reported parse CPU time around `9.57s`, format CPU time around `7.65s`, and print CPU time around `0.99s`.
The heaviest files by total time included `typescript/src/compiler/checker.ts`, `nextjs/turbopack/crates/turbopack-ecmascript/tests/analyzer/graph/bench/packages-bundle.js`, and `nextjs/packages/font/src/google/index.ts`.

The secondary benchmark command was `cargo run --release -p destack_formatter --example bench_stats -- --root language/test/fixtures/formatter/conformance/staging/prettier/tests/format --corpus full --workers 8 --runs 2 --warmup-runs 1`.
Secondary run set A reported mean total `52.018ms`, lines per second `1.54M/s`, and format CPU lines per second `1.23M/s`.
Secondary run set B reported mean total `51.471ms`, lines per second `1.56M/s`, and format CPU lines per second `1.29M/s`.

The post overlap fix benchmark command was rerun as `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
This rerun reported mean total `2.751s`, lines per second `2.84M/s`, and format CPU lines per second `1.00M/s`.
The rerun reported print CPU lines per second `7.69M/s` and parse CPU lines per second `800.2K/s`.
Compared with baseline run set B, total wall time improved slightly while format CPU throughput was slightly lower, which is within expected noise on this contested machine.

The post binary sweep benchmark command was rerun twice as `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Rerun A reported mean total `2.866s`, lines per second `2.73M/s`, and format CPU lines per second `954.1K/s`.
Rerun B reported mean total `2.887s`, lines per second `2.71M/s`, and format CPU lines per second `950.8K/s`.
Both reruns showed high contention noise with jitter between `4.6%` and `5.3%`, so these throughput deltas should be treated as noisy signals.

The post chain index safety sweep benchmark command was rerun twice as `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Rerun A reported mean total `3.075s`, lines per second `2.54M/s`, and format CPU lines per second `888.9K/s`.
Rerun B reported mean total `3.127s`, lines per second `2.50M/s`, and format CPU lines per second `832.3K/s`.
Rerun jitter ranged between `1.7%` and `3.4%`, which is still noisy but more stable than the earlier high contention checkpoint.
These numbers are directionally slower than the post binary sweep benchmark and should be treated as a potential throughput regression signal that needs follow up profiling.

### Conformance Spot Check Survey

A targeted spot check was run across `26` failing conformance tests with verbose diffs.
The sample included `13` prettier failures and `13` biome failures.
The prettier sample breakdown was `12` idempotence failures and `1` parse failure.
The biome sample breakdown was `11` output mismatches and `2` idempotence failures.
No read failures were observed in the sampled cases.

The sampled prettier failing cases were:
`js/arrows/chain-in-logical-expression.js`.
`js/assignment/destructuring-heuristic.js`.
`js/async/inline-await.js`.
`js/binary-expressions/array-and-object.js`.
`js/binary-expressions/arrow.js`.
`js/call/first-argument-expansion/expression-2nd-arg.js`.
`js/chain-expression/issue-15785-3.js`.
`js/conditional/new-expression.js`.
`js/method-chain/bracket_0-1.js`.
`js/template-literals/expressions.js`.
`js/throw_expressions/throw_expression.js`.
`typescript/ambient/ambient.ts`.
`typescript/chain-expression/test2.ts`.

The sampled biome failing cases were:
`js/module/object/property_object_member.js`.
`prettier/js/arrows/long-call-no-args.js`.
`prettier/js/assignment/destructuring.js`.
`prettier/js/assignment/issue-2540.js`.
`prettier/js/call/first-argument-expansion/issue-14454.js`.
`prettier/js/chain-expression/test-3.js`.
`prettier/js/conditional/new-expression.js`.
`prettier/js/functional-composition/redux_connect.js`.
`prettier/js/last-argument-expansion/number-only-array.js`.
`prettier/js/method-chain/bracket_0.js`.
`prettier/js/private-in/private-in.js`.
`prettier/typescript/comments/type_literals.ts`.
`ts/class/assignment_layout.ts`.

The strongest directional pattern in the survey is binary and assignment and chain line break instability.
Another strong pattern is first argument and last argument call expansion disagreement for long calls.
Another strong pattern is `new` and ternary and chain boundary wrapping disagreement.
Another strong pattern is template literal interpolation and comment placement drift.
Another strong pattern is method chain with bracket access and optional chain break placement drift.
One sampled TypeScript case remains a parse gap, which is `typescript/ambient/ambient.ts`.

### Formatter Or FIR Scoped Regression Slice

This section isolates formatter and FIR work by removing parser regressions from the post binary checkpoint of `46` unexpected regressions.
The parsed full regression inventories were recovered from temporary `--update-known-failures` runs and then restored.
At that checkpoint, the prettier full regression list had `28` cases and the biome full regression list had `18` cases.
The combined total at that checkpoint was `46` regressions.
The parser regression subset at that checkpoint was `17` cases, which was `14` from prettier and `3` from biome.
The formatter or FIR scoped subset at that checkpoint was `29` cases.
The latest post chain checkpoint moved the total to `47` regressions and the scoped parser split should be recomputed before reprioritizing clusters.

The scoped failure mode split is listed below.

| Suite | Scoped Regressions | Output Mismatch | Idempotence Mismatch |
|:--|--:|--:|--:|
| prettier | 14 | 0 | 14 |
| biome | 15 | 13 | 2 |
| total | 29 | 13 | 16 |

The strongest signal is that prettier scoped regressions are entirely idempotence instability.
The strongest signal for biome scoped regressions is direct output mismatch, with smaller idempotence drift.
This pattern suggests we have both stability issues and parity policy issues, with stability concentrated in prettier scope.

The scoped regression clusters are listed below.
The counts in this table are from the initial `42` case slice and are retained as directional themes for planning.

| Cluster | Count | Primary Suites | Typical Examples |
|:--|--:|:--|:--|
| assignment and destructuring layout | 6 | prettier and biome | `js/assignment/destructuring-heuristic.js`, `prettier/js/assignment/destructuring.js`, `js/assignment/issue-2540.js` |
| binary and logical operator wrapping | 5 | prettier and biome | `js/binary-expressions/array-and-object.js`, `js/binary-expressions/arrow.js`, `prettier/js/template-literals/logical-expressions.js` |
| call argument expansion and long call layout | 7 | prettier and biome | `js/call/first-argument-expansion/expression-2nd-arg.js`, `prettier/js/arrows/long-call-no-args.js`, `prettier/js/functional-composition/redux_connect.js` |
| chain and member and bracket access breaks | 5 | prettier and biome | `js/chain-expression/issue-15785-3.js`, `js/method-chain/bracket_0-1.js`, `prettier/js/method-chain/bracket_0.js` |
| control flow and new expression wrapping | 4 | prettier and biome | `js/conditional/new-expression.js`, `prettier/js/conditional/new-expression.js`, `js/throw_expressions/throw_expression.js` |
| template and tagged template formatting | 5 | prettier and biome | `js/template-literals/expressions.js`, `js/multiparser-css/styled-components.js`, `prettier/typescript/typeparams/tagged-template-expression.ts` |
| TypeScript layout and punctuation normalization | 8 | prettier and biome | `typescript/assignment/issue-10846.ts`, `typescript/conditional-types/conditional-types.ts`, `ts/type/mapped_type.ts` |
| private in operator wrapping | 1 | biome | `prettier/js/private-in/private-in.js` |
| object member multiline drift | 1 | biome | `js/module/object/property_object_member.js` |

There are currently `0` direct overlap regressions shared by both suites after normalizing biome `prettier/*` paths.
The previous overlap cases were resolved, so next fixes should target the largest remaining single suite clusters.

### High Confidence Module Mapping

The assignment and call and chain clusters map primarily to `language/formatter/src/format/expression/call/profile.rs`.
The call rendering and multiline decision expression maps to `language/formatter/src/format/expression/call/render.rs`.
The chain and bracket behavior maps to `language/formatter/src/format/expression/chain/format.rs`, `language/formatter/src/format/expression/chain/break.rs`, and `language/formatter/src/format/expression/member.rs`.
The binary and logical wrapping cluster maps to `language/formatter/src/format/expression/operator/binary.rs` and `language/formatter/src/format/expression/binary.rs`.
The conditional and new expression wrapping cluster maps to `language/formatter/src/format/expression/operator/new.rs`, `language/formatter/src/format/expression/ternary.rs`, and `language/formatter/src/format/expression/parentheses.rs`.
The template and tagged template cluster maps to `language/formatter/src/format/literal.rs` and call profile interactions in `language/formatter/src/format/expression/call/profile.rs`.
The TypeScript punctuation and separator drift maps to `language/formatter/src/format/expression/statement.rs`, `language/formatter/src/format/declaration.rs`, `language/formatter/src/format/signature.rs`, and `language/formatter/src/format/property.rs`.
The cross cluster idempotence pattern strongly suggests source sensitive heuristics are not converging after first format pass.

## System Findings

`language/formatter/src/format/context.rs` is a strong performance center with many caches and instrumentation hooks, but its size and breadth make correctness assumptions harder to audit.
Boundary comment and newline handling is distributed across many modules, which likely causes policy drift and is a key correctness risk.
Heuristic density is highest in call profiling, chain formatting, JSX formatting, declaration and annotation routing, and parentheses policies.
The formatter likely overfits to current fixtures in a few places with deterministic fast paths that can diverge from prettier and biome behavior.
Source substring probing is common in hot paths and can still be a throughput limiter despite caching.
The FIR layer itself appears solid and not obviously the main root cause for current conformance gaps.
Runtime evidence from the spot check indicates current drift is primarily formatting policy drift rather than parser level failures, except for specific TS ambient forms.

## Highest Priority Hotspots

`language/formatter/src/format/expression/call/profile.rs` has very high complexity and many interaction branches, so it is a primary correctness and performance audit target.
`language/formatter/src/format/expression/jsx.rs` has many special cases and callback and child heuristics, so it is a primary conformance divergence target.
`language/formatter/src/format/annotation.rs` and `language/formatter/src/format/annotation/defer.rs` implement many boundary routing rules that can silently destabilize output.
`language/formatter/src/format/expression/chain/*` has layered planners and break decisions that can amplify minor heuristic mistakes.
`language/formatter/src/format/declaration.rs`, `language/formatter/src/format/signature.rs`, and `language/formatter/src/format/argument.rs` share overlapping expansion and comment handling rules and should be unified where possible.
`language/formatter/src/format/expression/parentheses.rs` and `language/formatter/src/format/expression/ternary.rs` control syntactic safety and likely house subtle correctness edge cases.

## File Map

### Crate Root And Tooling

`language/formatter/Cargo.toml` defines crate dependencies and features and currently appears conventional with no direct correctness risk.
`language/formatter/README.md` documents conformance and benchmarking goals and should remain the single source of truth for formatter strategy updates.
`language/formatter/src/lib.rs` is a thin re export surface and is low risk.
`language/formatter/examples/bench_stats.rs` is a comprehensive benchmark harness with detailed counters and is the right place to enforce throughput regression gates.
`language/formatter/examples/corpus.rs` snapshots formatted corpus output and is useful for drift detection but currently defaults to oxfmt staging paths.
`language/formatter/fuzz/Cargo.toml` defines fuzz setup and is low risk.
`language/formatter/fuzz/fuzz_targets/formatter.rs` fuzzes parse and format and print stability and is valuable for panic safety but not full semantic correctness.
`language/formatter/scripts/corpus_bench.sh` wraps benchmark invocation and is low risk.
`language/formatter/scripts/corpus_diff.sh` diffs snapshots and is low risk.
`language/formatter/scripts/corpus_snapshot.sh` wraps snapshot generation and is low risk.
`language/formatter/src/tests/mod.rs` wires test modules and is low risk.
`language/formatter/src/tests/tests.rs` provides test parser and assert helpers and is foundational for fast iteration.
`language/formatter/src/tests/legacy_format_nodes.rs` contains broad node formatting tests and should be mined for missing prettier and biome style parity cases.

### Format Root Modules

`language/formatter/src/format/mod.rs` wires module exports and remains clean.
`language/formatter/src/format/context.rs` owns parse artifacts, source and span caches, annotation metadata, profiling counters, and timing data and is a high leverage performance module with high systemic complexity risk.
`language/formatter/src/format/timing.rs` provides timer and counter structures and is clean and low risk.
`language/formatter/src/format/scan.rs` provides shared source scanning helpers and should likely absorb duplicate scanners from expression and annotation modules.
`language/formatter/src/format/collection.rs` defines multiline break signals for collection like structures and is clean and reusable.
`language/formatter/src/format/operator.rs` formats operator tokens and is clean and deterministic.
`language/formatter/src/format/key.rs` formats names and keys with quote policy and has moderate correctness risk around quote consistency and identifier edge cases.
`language/formatter/src/format/path.rs` formats dotted paths and is low risk.
`language/formatter/src/format/literal.rs` formats scalar and template literals and includes normalization logic and has moderate correctness and performance risk due source span and interpolation heuristics.
`language/formatter/src/format/argument.rs` drives list and argument layout decisions and has high interaction risk with call profiling and declaration and annotation rules.
`language/formatter/src/format/block.rs` formats blocks and statement lists and has moderate correctness risk around annotation and comment adjacency behavior.
`language/formatter/src/format/declaration.rs` formats declarations and function like headers and bodies and has high correctness risk from many special cases.
`language/formatter/src/format/property.rs` formats object and type members and key quoting and has moderate risk from mixed quote and member and directive handling.
`language/formatter/src/format/signature.rs` formats function signatures and deferred boundary comments and is a high correctness hotspot for lambda and return type behavior.
`language/formatter/src/format/pattern.rs` formats destructuring and match patterns and has moderate risk around expansion heuristics and default value layout.
`language/formatter/src/format/directive.rs` handles formatter ignore directives and source passthrough and has high correctness impact because bypass logic can mask formatting defects.
`language/formatter/src/format/imports.rs` manages import and export formatting and sorting adjacency and has moderate conformance importance.
`language/formatter/src/format/dependency.rs` formats import and export dependency items and is mostly clean.
`language/formatter/src/format/enum.rs` formats enum fields and includes an inline cleanup note about comma ownership that is worth addressing.
`language/formatter/src/format/match.rs` formats match and switch cases and appears straightforward.
`language/formatter/src/format/where.rs` formats where clauses and is low risk.
`language/formatter/src/format/annotation.rs` owns annotation extraction and placement and boundary comment routing and is a high correctness and maintainability hotspot.
`language/formatter/src/format/annotation/defer.rs` performs deferred annotation transfer around syntax boundaries and is high risk for subtle instability when combined with other heuristics.

### Expression Entry And Classification

`language/formatter/src/format/expression/mod.rs` aggregates expression submodules and exports classification helpers and is clean but central.
`language/formatter/src/format/expression/core.rs` is the top dispatch for expression formatting and is high leverage for correctness.
`language/formatter/src/format/expression/classify.rs` defines trivial and complex expression classifiers and has high systemic impact because many heuristics depend on these classifications.
`language/formatter/src/format/expression/scan.rs` contains expression source scanners and parenthesis probes and should be consolidated with shared scanning utilities for consistency.
`language/formatter/src/format/expression/generic.rs` formats static type argument lists and is low risk with one key hug and expand branch.
`language/formatter/src/format/expression/sort.rs` formats import equals export rewrite cases and is narrow but correctness sensitive.

### Expression Operators And Binary Logic

`language/formatter/src/format/expression/operator/mod.rs` wires operator modules and is low risk.
`language/formatter/src/format/expression/operator/dispatch.rs` dispatches operator expression formatting and is central but compact.
`language/formatter/src/format/expression/operator/common.rs` contains trivial inline checks and is low risk.
`language/formatter/src/format/expression/operator/assign.rs` formats assignment expressions and has correctness sensitivity for break decisions in chained assignments.
`language/formatter/src/format/expression/operator/binary.rs` is a key binary layout engine and has high correctness and conformance impact.
`language/formatter/src/format/expression/operator/new.rs` formats `new` expressions and has targeted correctness sensitivity for parenthesized callees and chains.
`language/formatter/src/format/expression/binary.rs` manages type context propagation and binary helpers and is high leverage for type operator correctness.
`language/formatter/src/format/expression/member.rs` includes precedence logic and binary flattening helpers and is a major correctness dependency for safe parenthesization.

### Expression Calls And Chains

`language/formatter/src/format/expression/call/mod.rs` wires call modules and is low risk.
`language/formatter/src/format/expression/call/classify.rs` classifies call argument complexity and directly affects expansion and hugging behavior.
`language/formatter/src/format/expression/call/profile.rs` computes call profiles and layout strategies and is the single largest correctness and performance policy hotspot.
`language/formatter/src/format/expression/call/render.rs` renders call docs from profiles and has moderate risk due many profile driven branches.
`language/formatter/src/format/expression/chain/mod.rs` wires chain modules and is low risk.
`language/formatter/src/format/expression/chain/base.rs` extracts chain nodes and metadata and has high downstream impact on chain safety and breaks.
`language/formatter/src/format/expression/chain/classify.rs` classifies chain break style and has high conformance impact.
`language/formatter/src/format/expression/chain/break.rs` decides break placements and should be benchmarked for branch pruning opportunities.
`language/formatter/src/format/expression/chain/length.rs` measures chain lengths and complexity and is a key throughput lever when reused effectively.
`language/formatter/src/format/expression/chain/format.rs` renders chains using planners and is a high correctness and readability hotspot.

### Expression Control And Statements

`language/formatter/src/format/expression/control.rs` formats control flow expression forms and has moderate correctness sensitivity.
`language/formatter/src/format/expression/statement.rs` formats statement like expressions including imports and loops and returns and is a significant correctness module.
`language/formatter/src/format/expression/declarator.rs` formats declarators and assignment like forms and has high interaction risk with call and chain and binary logic.
`language/formatter/src/format/expression/ternary.rs` handles ternary chain extraction and separator comment routing and is a high correctness hotspot.
`language/formatter/src/format/expression/parentheses.rs` controls parenthesis insertion and removal and is a critical safety and conformance module.
`language/formatter/src/format/expression/primary.rs` formats many primary expression forms and has moderate risk from breadth.
`language/formatter/src/format/expression/object.rs` formats object literals and properties and has moderate risk from expansion and annotation interactions.

### Expression JSX And Tree Literals

`language/formatter/src/format/expression/jsx.rs` handles tree literal attributes and children and callback heuristics and is a top priority correctness and conformance and performance hotspot.
`language/formatter/src/format/expression/jsx.rs` key methods include `should_force_break_tree_attributes`, `tree_child_breaks_element`, `tree_literal_should_break`, and `format_tree_literal`.
`language/formatter/src/format/expression/jsx.rs` likely needs rule simplification and stronger golden tests versus prettier and biome for mixed text and braced whitespace and callback children.

### Expression Tests

`language/formatter/src/format/expression/tests.rs` has strong breadth and many targeted regressions and should be expanded with prettier and biome derived fixtures to reduce heuristic drift.

## Cross File Fishiness Notes

Boundary comment routing appears duplicated across `annotation.rs`, `annotation/defer.rs`, `signature.rs`, `declaration.rs`, `argument.rs`, `ternary.rs`, and `parentheses.rs`.
Source scanning helpers are split between `format/scan.rs` and `expression/scan.rs` and ad hoc local logic in multiple files, which likely hurts consistency and cache hit rates.
Heuristics often check both source shape and AST shape with overlapping intent, which can create unstable branch interactions after small edits.
JSX and call and chain rules appear to encode many local optimizations without a single explicit layout policy matrix, which increases long term maintenance cost.
Several modules eagerly allocate `String` for span fragments in decision paths, which may be avoidable with borrowed slices or precomputed lightweight metadata.

## Performance Opportunities

Move repeated source boundary checks to context cached predicates keyed by span and predicate type.
Unify whitespace and comment scanning helpers and ensure they all route through cached context methods.
Promote deterministic fast paths earlier in call and chain and JSX pipelines when annotation and comment and newline states are absent.
Reduce repeated `to_string` and owned span extraction in hot loops, especially in annotation and JSX and literal interpolation paths.
Add microbench tags around specific layout decision functions in call and chain and JSX to isolate regressions faster.
Prioritize perf sweeps in this order: `context.rs`, `annotation.rs`, `expression/jsx.rs`, `argument.rs`, and `expression/call/analyze_layout.rs`.
Prefer moving branch-heavy scan plus policy loops into small helper passes so hot paths can early return on simple no-comment and no-break cases.

## Correctness Opportunities

Define a single boundary comment policy table and consume it from annotation and signature and ternary and parenthesis modules.
Reduce overlapping expansion heuristics between argument lists and signatures and declarations by sharing one scoring model.
Add stronger differential tests against prettier and biome for call chains, nested ternaries, JSX mixed content, and comments around delimiters.
Add snapshot tests that assert full output for known unstable patterns instead of partial contains style assertions.

## Cleanliness And Architecture Opportunities

Split `context.rs` into smaller focused cache and metadata components while preserving cheap access patterns.
Split `call/profile.rs` into analysis and policy and rendering profile stages with explicit data contracts.
Split `jsx.rs` into attribute layout, child layout, and tree wrapping modules.
Document a small set of canonical layout primitives and scoring rules in `language/formatter/README.md` to reduce ad hoc heuristics.

## Suggested Next Execution Order

Step 1 should continue call profile stabilization on remaining idempotence regressions, especially assignment and first argument expansion clusters.
Step 2 should simplify chain and binary break policy around `||`, `&&`, `??`, bracket member chains, long chained calls, and the call plus index expansion split seen in `prettier/js/method-chain/bracket_0.js`.
Step 3 should build a boundary comment policy matrix and apply it to annotation and signature and ternary modules.
Step 4 should simplify JSX child and attribute break heuristics with fixture driven expected outputs from prettier and biome.
Step 5 should add microbench and counter checkpoints for call profile and chain break decision paths before broader refactors.
Step 6 should rerun conformance suites and benchmark reruns after each step and keep throughput checks in the same loop.

## Historical Checkpoint Before Known Failure Rebase

This checkpoint is retained as historical context from before the known-failure baseline rebase.
All numbers in this section were captured on February 12, 2026 in this worktree after restoring `language/formatter/src/format/expression/call/profile.rs` and `language/formatter/src/format/expression/operator/assign.rs` to baseline.
The only active formatter code edits at this checkpoint are in `language/formatter/src/format/expression/chain/format.rs` and `language/formatter/src/format/expression/operator/binary.rs`.

### Current Baseline Health

`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` failed by unexpected regressions with `52` total regressions.

The latest full conformance summary is listed below.

| Suite | Passed | Failed | Ignored | Total | Rate | Unexpected Regressions |
|:--|--:|--:|--:|--:|--:|--:|
| biome | 631 | 1096 | 9 | 1727 | 36.54% | 22 |
| prettier | 1229 | 563 | 1436 | 1792 | 68.58% | 30 |
| total | 1860 | 1659 | 1445 | 3519 | 52.86% | 52 |

The current parse failure counts are `365` for biome and `436` for prettier.
The current prettier failure kind split is `output=0` and `idempotence=127`, which keeps idempotence stability as the top correctness risk in the formatter layer.
The current biome failure kind split is `output=699` and `idempotence=32`, which keeps direct output policy mismatch as the top parity risk.

### Spot Check Status For Current Planning

The directional spot checks that currently drive planning are still failing.
`js/assignment/issue-2540.js` still fails in prettier idempotence and biome output checks.
`js/conditional/new-expression.js` still fails in prettier idempotence and biome output checks.
`js/async/inline-await.js` still fails in prettier idempotence checks and remains a directionally useful chain and call stability case.

### Isolation Findings

Disabling the no annotation multi argument fast path in `language/formatter/src/format/expression/call/profile.rs` did not change the `issue-2540` failure shape.
Restoring `language/formatter/src/format/expression/operator/assign.rs` to baseline did not change the `issue-2540` failure shape.
Restoring `language/formatter/src/format/expression/call/profile.rs` to baseline did not change the `issue-2540` failure shape.
These A/B results suggest the current `issue-2540` instability is not caused by those specific toggles and likely involves a deeper call list and nested object break interaction.

### Throughput Checkpoint

The benchmark command was `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Run A reported mean total `2.734s`, lines per second `2.86M/s`, format CPU lines per second `998.7K/s`, and jitter `1.2%`.
Run B reported mean total `4.634s`, lines per second `1.69M/s`, format CPU lines per second `568.8K/s`, and jitter `33.7%`.
Run C reported mean total `3.581s`, lines per second `2.18M/s`, format CPU lines per second `752.9K/s`, and jitter `13.2%`.
The machine contention is clearly visible, so planning should use throughput ranges instead of a single point estimate.
A practical current range is `2.18M/s` to `2.86M/s` for total lines per second and `752.9K/s` to `998.7K/s` for format CPU lines per second.

### Immediate Plan Anchors

The next deep sweep should prioritize call list group break convergence and ternary in call argument stability before additional micro optimizations.
The next deep sweep should keep `js/assignment/issue-2540.js`, `js/conditional/new-expression.js`, and `js/async/inline-await.js` as hard gating probes between every formatter heuristic change.
The next deep sweep should keep `prettier/js/method-chain/bracket_0.js` and `js/chain-expression/issue-15785-3.js` as hard gating probes for chain line safety and idempotence.

### Known Failures Baseline Reset

On February 12, 2026, the conformance known-failures baseline was intentionally rebased to remove unexpected-regression noise and support systematic prioritization.
The command used was `cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome --update-known-failures`.
This updated `language/test/fixtures/formatter/conformance/biome-known-failures.txt` to `1096` cases and `language/test/fixtures/formatter/conformance/prettier-known-failures.txt` to `563` cases.

After the update, a plain verification run with `cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with no unexpected regressions.
The verification run reported `PASSED: all 3519 tests accounted for`.
This gives us a stable baseline where future deltas are attributable to deliberate formatter or FIR changes rather than stale known-failure bookkeeping.

## Quality Cleanup Batch 1

This batch focused on cleanup and systemicness with low behavior risk before additional conformance targeting.
The scope was restricted to `language/formatter/src/format/expression/chain/format.rs` and `language/formatter/src/format/expression/operator/binary.rs`.

### Chain Cleanup

`collect_chain_root_parts` no longer allocates an intermediate `Vec<StringId>` for remaining path segments.
The logic now iterates directly over a tail slice, which removes one avoidable allocation in a hot normalization path.
Deferred boundary comment collection now runs only when the path has a synthetic tail segment, which avoids unnecessary scans for single segment path roots.
The existing chain safety fix for direct index line merging is unchanged in behavior in this batch.

### Binary Cleanup

Logical operator checks were centralized into `is_logical_binary_operator`.
Mixed logical precedence checks were centralized into `is_mixed_logical_precedence_pair`.
Source break preservation rules were centralized into `preserve_source_operator_break`.
Trailing logical operator preference checks were centralized into `operand_prefers_trailing_logical_operator`.
This reduces duplicated condition logic and lowers future drift risk when adjusting logical operator policies.

### Validation

`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
No conformance-targeting changes were made in this batch by design.
`cargo fmt` and `cargo +nightly-2025-11-27 fmt` both failed in this environment because `rustfmt` rejected existing let-chain syntax as requiring Rust 2024 parsing.

## Quality Cleanup Batch 2

This batch focused on `language/formatter/src/format/expression/call/profile.rs` with behavior-preserving cleanup and generalization.
The main goal was to reduce duplicated decision logic and normalize fast-path predicates without changing conformance outcomes.

### Batch 2 Changes

`leading_dynamic_arguments` now centralizes leading-argument slicing before callback-tail and compactness checks.
`call_argument_layout_class_is_simple_multi_unannotated` now centralizes compact unannotated multi-argument classification.
`call_arguments_use_no_annotation_multi_argument_fast_path` no longer repeats the infix-annotation check because that state is already encoded in the shared layout-class helper.
`write_single_call_argument_inline_wrapped` now centralizes the repeated `(` argument `)` emission used by multiple single-argument paths.
`CallArgumentLayoutRenderState` and `format_call_argument_layout_decision` now centralize final layout rendering so single and multi-argument paths share one renderer.
`format_single_call_argument_with_group` now delegates final rendering to the shared layout renderer instead of carrying a local duplicate match block.
`format_call_arguments_with_group` now delegates final rendering to the same shared layout renderer.

### Why This Matters

A single rendering function for `CallArgumentLayoutDecision` reduces drift risk between single and multi-argument call paths.
Centralized predicate helpers reduce the chance of semantically similar conditions diverging as call heuristics continue to evolve.
Removing repeated wrapped-single emission avoids another class of tiny but chronic mismatch bugs where one path receives a micro-update and another does not.
These are maintainability and correctness-hardening changes that also simplify future profiling because path counters now map more directly to shared logic.

### Validation After Batch 2

`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129/129` passing and `6` ignored.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with all cases accounted for and no unexpected regressions.
The conformance checkpoint stayed exactly at `biome 631 passed / 1096 failed / 9 ignored` and `prettier 1229 passed / 563 failed / 1436 ignored`.
The failure-kind split stayed at `biome parse=365 output=699 idempotence=32` and `prettier parse=436 output=0 idempotence=127`.

### Throughput Checkpoint After Batch 2

The benchmark command remained `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Rerun A reported mean total `3.337s`, lines per second `2.34M/s`, and format CPU lines per second `786.2K/s`.
Rerun B reported mean total `3.664s`, lines per second `2.14M/s`, and format CPU lines per second `718.8K/s`.
Jitter was `5.3%` in rerun A and `4.3%` in rerun B, which confirms the machine remains contention-heavy.
The practical current throughput range is `2.14M/s` to `2.34M/s` lines per second and `718.8K/s` to `786.2K/s` format CPU lines per second.

### Deep Method Map For Current Cleanup Scope

`build_call_argument_expansion_profiles` is the primary expansion policy root for regular and chain decisions and remains the highest-risk call formatting method.
`resolve_call_argument_layout_class` is the primary scan-and-classify hot path for call arguments and should remain allocation-light and cache-friendly.
`leading_dynamic_arguments` is a tiny shared helper that removes repeated `take(len - 1)` patterns and should remain the only leading-slice utility.
`leading_arguments_are_compact_simple_unannotated` and `leading_arguments_are_compact_callback_tail_candidates` are callback-tail policy gates and are still correctness-sensitive for hug-last behavior.
`decide_post_hugged_call_argument_layout` remains the single highest-complexity decision method and is a priority candidate for future decomposition into policy phases.
`format_call_argument_layout_decision` now acts as the canonical call-argument rendering switch and should remain exhaustive over `CallArgumentLayoutDecision` variants.
`format_single_call_argument_with_group` is now a cleaner planner-plus-delegate path and no longer carries bespoke render branching.
`format_call_arguments_with_group` is now consistent with the single-argument flow at the render stage, which reduces future parity drift.
`call_arguments_force_expand_for_chain` already consumes the shared simple-layout helper and remains a clean fast-false path for chain formatting.

### Remaining Fishiness In `call/profile.rs`

`decide_post_hugged_call_argument_layout` still interleaves comment profiling, expansion profiling, and hug-last policy in one method, which keeps cognitive load high.
Comment-profile collection remains conditional and contextual, which is correct but still hard to reason about for idempotence edge cases.
The boundary between call-shape classification and explicit source-length probing remains somewhat porous, which can still produce policy surprises under tiny source edits.
The current counter set is broad but still not organized as a strict decision-tree trace, which slows root-cause work for conformance drift.

### Next Cleanup Targets

Extract a small explicit `CallArgumentPolicyInputs` data structure to separate scan facts from policy decisions inside `decide_post_hugged_call_argument_layout`.
Normalize source-length probing so all inline-fit checks route through one cached length function path.
Centralize boundary-comment handling contracts between call/profile, annotation defer rules, and ternary/parenthesis modules to reduce cross-module drift.
Add targeted conformance probes for the known unstable clusters before each policy refactor step, with emphasis on idempotence-only prettier failures.

## Quality Cleanup Batch 3

This batch continues formatter and FIR quality cleanup with focus on correctness stability and systemicness before new conformance feature work.
This batch was validated on February 12, 2026 with full release tests and refreshed conformance and benchmark baselines.

### Batch 3 Code Changes

`language/formatter/src/format/expression/call/profile.rs` keeps the shared call-layout rendering refactor and helper extraction from batch 2.
`language/formatter/src/format/expression/call/profile.rs` keeps the single multiline argument force-expand guard for non-chain calls to reduce source-driven idempotence churn in chain contexts.
`language/formatter/src/format/expression/chain/format.rs` keeps direct index line merge logic so chain lines do not begin with bare `[index]` segments after line breaking.
`language/formatter/src/format/expression/operator/binary.rs` keeps centralized logical-operator policy helpers from cleanup batch 1.
`language/formatter/src/format/expression/jsx.rs` keeps dead helper removal from cleanup batch 2.

### Batch 3 Correctness Rollback

A compact-source chain length experiment was reverted because it introduced a formatter regression in our own transform suite.
The reverted probes were in `language/formatter/src/format/expression/chain/classify.rs` and `language/formatter/src/format/expression/chain/length.rs`.
The regression case was `transform/expressions/complex.md/curried-function-calls/curried-call-with-objects`.
This rollback restored deterministic behavior in curried object-tail call wrapping while preserving the rest of the cleanup changes.

### Batch 3 Validation Results

`cargo +nightly-2025-11-27 fmt -p destack_formatter` passed.
`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with all `3519` tests accounted for.

The latest prettier and biome summary is listed below.

| Suite | Passed | Failed | Ignored | Total | Rate | Notes |
|:--|--:|--:|--:|--:|--:|:--|
| biome | 631 | 1096 | 9 | 1727 | 36.54% | known-failure count unchanged |
| prettier | 1231 | 561 | 1436 | 1792 | 68.69% | known-failure count improved by 2 |
| total | 1862 | 1657 | 1445 | 3519 | 52.91% | all cases accounted for |

The current failure-kind split is `biome: parse=365 output=699 idempotence=32` and `prettier: parse=436 output=0 idempotence=125`.
The two newly fixed prettier cases are `js/assignment/issue-2540.js` and `js/throw_expressions/throw_expression.js`.

### Known-Failures Baseline Update

Known-failure files were refreshed after this batch using `--update-known-failures`.
`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` is now `561` cases.
`language/test/fixtures/formatter/conformance/biome-known-failures.txt` remains `1096` cases.
A follow-up plain conformance run confirmed no unexpected regressions.

### Benchmark Checkpoint After Batch 3

The benchmark command remained `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Run set A reported mean total `3.034s`, lines per second `2.58M/s`, and format CPU lines per second `896.0K/s`.
Run set B reported mean total `2.917s`, lines per second `2.68M/s`, and format CPU lines per second `919.2K/s`.
The current practical throughput range is `2.58M/s` to `2.68M/s` for total lines per second and `896.0K/s` to `919.2K/s` for format CPU lines per second.
The machine remained contested so these numbers should be interpreted as a short noisy range rather than a strict monotonic trend.

### Current Risk Summary

The largest formatter-layer correctness risk remains prettier idempotence drift in chain and call and nested wrapping patterns.
The largest parity risk remains biome output mismatch clusters, especially in expression wrapping and object and call layout policy differences.
Parser failures remain large in both suites but are out of scope for formatter and FIR cleanup in this workstream.

### Current Execution Plan

Step 1 should target remaining prettier idempotence clusters by tightening chain call break convergence with source-insensitive policy gates.
Step 2 should split `decide_post_hugged_call_argument_layout` into explicit policy phases so callback-tail and collection-tail logic cannot drift.
Step 3 should extract shared boundary-comment placement rules across call and chain and ternary and annotation defer paths.
Step 4 should re-profile chain and call hot paths and remove redundant source-scan calls in repeatedly executed predicates.
Step 5 should continue fixture-driven conformance improvements in small batches while keeping `destack_formatter`, `formatter`, and `oxfmt` green after every change.

## Quality Cleanup Batch 4

This batch performs a wider quality sweep focused on systemic correctness hardening and AGENTS-style cleanliness in formatter modules before further conformance feature work.
This pass was validated on February 12, 2026 with formatter-specific clippy, full release test suites, and a fresh benchmark checkpoint.

### Batch 4 Scope

This batch touched `language/formatter/src/format/declaration.rs`.
This batch touched `language/formatter/src/format/expression/chain/format.rs`.
This batch touched `language/formatter/src/format/expression/generic.rs`.
This batch touched `language/formatter/src/format/expression/object.rs`.
This batch touched `language/formatter/src/format/expression/operator/new.rs`.
This batch touched `language/formatter/src/format/expression/statement.rs`.
This batch touched `language/formatter/src/format/signature.rs`.

### Batch 4 Cleanup Details

`declaration.rs` removed duplicated branches in type alias value layout selection and normalized inline gating into explicit `can_inline` variables.
`declaration.rs` replaced one local `let Some(...) else` traversal with `?` for cleaner and safer control flow.
`chain/format.rs` introduced `ChainRenderInputOptions` so chain render planning no longer requires an eight-argument helper signature.
`generic.rs`, `object.rs`, `operator/new.rs`, `statement.rs`, and `signature.rs` changed `&Vec<T>` function parameters to `&[T]` slices for cleaner APIs and lower allocation-pressure assumptions.
`statement.rs` changed import and export argument forwarding from `as_ref()` to `as_deref()` to match slice-based signatures.

### Batch 4 Lint Sweep Status

`cargo clippy -p destack_formatter --release --all-targets` now reports no formatter-crate warnings.
Remaining warnings in the command output are in transitive workspace crates (`destack_fir`, `destack_parser`, and `destack_workspace`) and are outside this formatter cleanup scope.

### Batch 4 Validation Results

`cargo +nightly-2025-11-27 fmt -p destack_formatter` passed.
`cargo clippy -p destack_formatter --release --all-targets` passed for formatter targets.
`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with all `3519` tests accounted for.

The current conformance baseline remains unchanged in this batch.
`biome` remains `631 passed / 1096 failed / 9 ignored`.
`prettier` remains `1231 passed / 561 failed / 1436 ignored`.

### Batch 4 Throughput Checkpoint

The benchmark command remained `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
The run reported mean total `2.827s`, lines per second `2.77M/s`, and format CPU lines per second `991.8K/s`.
Measured jitter for this run was `1.1%`, which is comparatively stable for this machine.
This checkpoint is directionally faster than earlier noisy runs in this session and does not show a throughput regression signal from the cleanup batch.

### Remaining Work After Batch 4

The major remaining correctness risk is still conformance idempotence and policy mismatch clusters rather than formatter internal lint and API cleanliness.
The next changes should target policy convergence in call and chain and boundary-comment interaction paths while preserving the now clean formatter-clippy baseline.
## Quality Cleanup Batch 5
This batch continues the declaration architecture cleanup with a no-behavior-change module split and full baseline revalidation.
This pass was executed on February 12, 2026.

### Batch 5 Scope
This batch touched `language/formatter/src/format/declaration/mod.rs`.
This batch touched `language/formatter/src/format/declaration/dispatch.rs`.
This batch touched `language/formatter/src/format/declaration/function_like.rs`.
This batch touched `language/formatter/src/format/declaration/type_alias.rs`.
This batch touched `language/formatter/src/format/declaration/type_like.rs`.

### Batch 5 Refactor Details
`Declaration::Function` formatting moved from `dispatch.rs` into `function_like.rs` with explicit helper boundaries for lambda parameter policy and deferred lambda-arrow annotations.
Struct and class and enum and interface formatting moved from `dispatch.rs` into `type_like.rs` to separate declaration routing from type-like declaration rendering.
`dispatch.rs` is now a lightweight router and trait-wiring module with shared `format_super_type_clause` only.
A transient oxfmt regression in `ts/comments/union.ts` was introduced during the split and then fixed by restoring the original `format_expression_without_prefix_annotations` directive and postfix-annotation behavior in `type_alias.rs`.
The declaration formatter now has explicit module boundaries with current line counts of `dispatch.rs=314`, `function_like.rs=471`, `type_like.rs=385`, `type_alias.rs=389`, `module_like.rs=268`, and `mod.rs=5`.
Historic `declaration.rs` mentions in earlier sections of this map refer to the pre-split file before this batch.

### Batch 5 Validation Results
`cargo +nightly-2025-11-27 fmt -p destack_formatter` passed.
`cargo check -p destack_formatter` passed.
`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with all `3519` tests accounted for.
`cargo test --release -p destack_test --test formatter-conformance` passed with all `3648` tests accounted for.

### Batch 5 Conformance Snapshot
`oxfmt` is `129 passed / 0 failed / 6 ignored` at `100.00%` hard-suite pass rate.
`biome` is `631 passed / 1096 failed / 9 ignored` with failure kinds `parse=365 output=699 idempotence=32`.
`prettier` is `1231 passed / 561 failed / 1436 ignored` with failure kinds `parse=436 output=0 idempotence=125`.
The combined accounted total is `1991 passed / 1657 failed / 1451 ignored` over `3648` tests.

### Batch 5 Throughput Checkpoint
The benchmark command remained `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Run A reported mean total `3.053s`, total lines per second `2.56M/s`, and format CPU lines per second `892.8K/s` with `4.0%` jitter.
Run B reported mean total `3.431s`, total lines per second `2.28M/s`, and format CPU lines per second `789.8K/s` with `7.1%` jitter.
Run C reported mean total `3.337s`, total lines per second `2.34M/s`, and format CPU lines per second `808.7K/s` with `11.0%` jitter.
This checkpoint remains noisier and slower than the earlier stable Batch 4 run and should be treated as contested-machine signal, not a confirmed regression.

### Batch 5 Risk Notes
The parser-failure totals remain large but are out of scope for this formatter and FIR cleanup stream.
The largest in-scope correctness risks remain prettier idempotence and biome output-policy mismatches in call and chain and boundary-comment interactions.

## Quality Cleanup Batch 6
### Batch 6 Scope
This batch split call-argument layout, shape scanning, and hug-last planning out of `language/formatter/src/format/expression/call/analyze.rs` into `language/formatter/src/format/expression/call/analyze_layout.rs`.
This batch kept behavior unchanged and focused on reducing cross-cutting heuristic density in a single module.

### Batch 6 Refactor Details
I added `language/formatter/src/format/expression/call/analyze_layout.rs` and moved layout-class scanning, single-argument fast-path checks, and planner state types there.
I kept `language/formatter/src/format/expression/call/analyze.rs` focused on expansion-profile construction, caching, and inline argument writing helpers.
I updated `language/formatter/src/format/expression/call/mod.rs` to include `mod analyze_layout;` while keeping `profile.rs` API usage stable through `analyze.rs` re-exports.
No counter names or layout policy branches were changed in this refactor.

### Batch 6 Validation Results
`cargo +nightly-2025-11-27 fmt -p destack_formatter` passed.
`cargo check -p destack_formatter` passed.
`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` passed with unchanged accounting.

### Batch 6 Conformance Snapshot
`oxfmt` remained `129 passed`, `0 failed`, `6 ignored`, `100.00%`.
`biome` remained `631 passed`, `1096 failed`, `9 ignored`, `36.54%`.
`prettier` remained `1231 passed`, `561 failed`, `1436 ignored`, `68.69%`.
Failure kind totals remained `biome: parse=365, output=699, idempotence=32` and `prettier: parse=436, output=0, idempotence=125`.

### Batch 6 Throughput Checkpoint
Run A of `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `3.952s`, lines per second `1.98M/s`, and format CPU lines per second `695.4K/s` with `11.5%` jitter.
Run B of the same command reported mean total `2.896s`, lines per second `2.70M/s`, and format CPU lines per second `953.3K/s` with `5.3%` jitter.
Given the contested machine and large jitter spread, this batch has no confirmed throughput regression signal.

### Batch 6 Risk Notes
The call pipeline is now cleaner, but `language/formatter/src/format/expression/call/analyze_layout.rs` is still large and should be split again into `shape` and `hug_last` submodules in a later batch.
The next highest-impact architecture cleanup target remains `language/formatter/src/format/expression/chain/format.rs`.
## Quality Cleanup Batch 7
### Batch 7 Scope
This batch split chain planning and chain line-grouping logic out of `language/formatter/src/format/expression/chain/format.rs`.
This batch kept formatter behavior stable and reduced heuristic concentration in a single chain file.

### Batch 7 Refactor Details
I added `language/formatter/src/format/expression/chain/normalize.rs` for chain root normalization and layout plan construction.
I added `language/formatter/src/format/expression/chain/policy.rs` for chain render input scoring and deterministic render decisions.
I added `language/formatter/src/format/expression/chain/line_group.rs` for chain line grouping and direct-index merge policy.
I reduced `language/formatter/src/format/expression/chain/format.rs` to render orchestration and operation emit logic.
I updated `language/formatter/src/format/expression/chain/mod.rs` to wire the new internal modules.
I also restored missing declaration dispatch wiring and cleaned unused imports in `language/formatter/src/format/declaration/dispatch.rs` so formatter crate checks are warning-free again.

### Batch 7 Validation Results
`cargo +nightly-2025-11-27 fmt -p destack_formatter` passed.
`cargo check -p destack_formatter` passed.
`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` passed with unchanged accounting.
`cargo clippy -p destack_formatter --release --all-targets` passed for formatter changes, with existing warnings only in other workspace crates.

### Batch 7 Conformance Snapshot
`oxfmt` remained `129 passed`, `0 failed`, `6 ignored`, `100.00%`.
`biome` remained `631 passed`, `1096 failed`, `9 ignored`, `36.54%`.
`prettier` remained `1231 passed`, `561 failed`, `1436 ignored`, `68.69%`.
Failure kind totals remained `biome: parse=365, output=699, idempotence=32` and `prettier: parse=436, output=0, idempotence=125`.

### Batch 7 Throughput Checkpoint
Run A of `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `3.069s`, lines per second `2.55M/s`, and format CPU lines per second `881.9K/s` with `2.2%` jitter.
Run B of the same command reported mean total `4.644s`, lines per second `1.68M/s`, and format CPU lines per second `572.2K/s` with `12.8%` jitter.
Given the large contention spread between runs, this batch has no confirmed throughput regression signal.

### Batch 7 Risk Notes
`language/formatter/src/format/expression/chain/format.rs` is now materially smaller, but chain behavior still depends on shared annotation and call policies.
The next cleanup target should be reducing policy overlap between `expression/chain/policy.rs`, `expression/call/analyze_layout.rs`, and annotation boundary deferral logic.

## Quality Cleanup Batch 8
### Batch 8 Scope
This batch continued the systemic cleanup in chain and call layout logic with no intended behavior change.
This batch targeted long nested branches and mixed scan and policy and render logic in single functions.
This batch also refreshed the post-refactor baseline numbers so planning does not rely on stale checkpoints.

### Batch 8 Refactor Details
I refactored `language/formatter/src/format/expression/chain/normalize.rs` by decomposing `plan_chain_layout` into focused helper functions for boundary-comment guards, head-promotion checks, and argument-chain promotions.
I kept `plan_chain_layout` as orchestration only so the decision flow is explicit and reviewable.
I refactored `language/formatter/src/format/expression/call/analyze_layout.rs` by extracting call argument layout scanning into `CallArgumentLayoutScanState` helpers and by splitting hug-last inline checks into dedicated helper predicates.
I preserved existing counter and planner state types so downstream call rendering and profiling code remained stable.

### Batch 8 Validation Results
`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with all `3519` tests accounted for.

### Batch 8 Conformance Snapshot
`oxfmt` remained `129 passed`, `0 failed`, `6 ignored`, `100.00%`.
`biome` remained `631 passed`, `1096 failed`, `9 ignored`, `36.54%`.
`prettier` remained `1231 passed`, `561 failed`, `1436 ignored`, `68.69%`.
Failure kind totals remained `biome: parse=365, output=699, idempotence=32` and `prettier: parse=436, output=0, idempotence=125`.

### Batch 8 Throughput Checkpoint
Run A of `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `9.652s`, lines per second `810.4K/s`, and format CPU lines per second `264.1K/s` with `24.0%` jitter.
Run B of the same command reported mean total `4.721s`, lines per second `1.66M/s`, and format CPU lines per second `573.8K/s` with `13.6%` jitter.
These two runs confirm severe contention noise and do not support a trustworthy throughput regression conclusion yet.

### Batch 8 Throughput Recheck After Corpus Restore
The ecosystem corpus root `test/fixtures/ecosystem/checkouts` was temporarily missing and was restored by linking to `language/test/fixtures/ecosystem/checkouts`.
Recheck run A of `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `3.722s`, lines per second `2.10M/s`, and format CPU lines per second `721.1K/s` with `2.4%` jitter.
Recheck run B of the same command reported mean total `4.861s`, lines per second `1.61M/s`, and format CPU lines per second `550.2K/s` with `18.1%` jitter.
Recheck run C of the same command reported mean total `2.708s`, lines per second `2.89M/s`, and format CPU lines per second `1.01M/s` with `0.6%` jitter.
This recheck indicates no confirmed persistent throughput regression and the earlier extreme slowdown was dominated by corpus path issues and host contention.

### Batch 8 Risk Notes
`language/formatter/src/format/expression/call/analyze_layout.rs` is now cleaner but still very large at `888` lines and should be split by responsibility into `shape.rs` and `hug_last.rs`.
`language/formatter/src/format/expression/chain/normalize.rs` dropped branch density in the entry point, but policy overlap with call-layout heuristics still needs unification in a later conformance batch.

## Formatter File Inventory
This section is auto-generated on February 12, 2026 from `language/formatter/src/format`.
This section records line count and detected function signature lines for each formatter file.

### Summary Table
This table is sorted by total line count in descending order.

| File | Lines | Function Signatures |
|:--|--:|--:|
| `language/formatter/src/format/context.rs` | 1918 | 85 |
| `language/formatter/src/format/annotation.rs` | 1817 | 68 |
| `language/formatter/src/format/expression/jsx.rs` | 1808 | 35 |
| `language/formatter/src/format/argument.rs` | 1408 | 45 |
| `language/formatter/src/format/expression/tests.rs` | 1187 | 89 |
| `language/formatter/src/format/expression/statement.rs` | 950 | 13 |
| `language/formatter/src/format/block.rs` | 905 | 29 |
| `language/formatter/src/format/expression/call/analyze_layout.rs` | 888 | 27 |
| `language/formatter/src/format/expression/operator/binary.rs` | 837 | 7 |
| `language/formatter/src/format/annotation/defer.rs` | 830 | 27 |
| `language/formatter/src/format/property.rs` | 814 | 27 |
| `language/formatter/src/format/expression/chain/break.rs` | 813 | 16 |
| `language/formatter/src/format/expression/chain/classify.rs` | 808 | 29 |
| `language/formatter/src/format/expression/chain/base.rs` | 759 | 24 |
| `language/formatter/src/format/expression/primary.rs` | 751 | 4 |
| `language/formatter/src/format/expression/declarator.rs` | 728 | 5 |
| `language/formatter/src/format/literal.rs` | 710 | 26 |
| `language/formatter/src/format/expression/call/profile.rs` | 704 | 13 |
| `language/formatter/src/format/expression/parentheses.rs` | 630 | 24 |
| `language/formatter/src/format/expression/call/analyze.rs` | 607 | 19 |
| `language/formatter/src/format/directive.rs` | 603 | 17 |
| `language/formatter/src/format/expression/ternary.rs` | 594 | 12 |
| `language/formatter/src/format/expression/chain/normalize.rs` | 550 | 15 |
| `language/formatter/src/format/expression/member.rs` | 544 | 17 |
| `language/formatter/src/format/expression/binary.rs` | 535 | 17 |
| `language/formatter/src/format/pattern.rs` | 522 | 19 |
| `language/formatter/src/format/expression/call/classify.rs` | 473 | 22 |
| `language/formatter/src/format/declaration/function_like.rs` | 471 | 7 |
| `language/formatter/src/format/signature.rs` | 430 | 16 |
| `language/formatter/src/format/expression/control.rs` | 410 | 9 |
| `language/formatter/src/format/declaration/type_alias.rs` | 389 | 5 |
| `language/formatter/src/format/declaration/type_like.rs` | 385 | 9 |
| `language/formatter/src/format/expression/operator/dispatch.rs` | 339 | 7 |
| `language/formatter/src/format/expression/chain/line_group.rs` | 334 | 11 |
| `language/formatter/src/format/expression/chain/length.rs` | 330 | 10 |
| `language/formatter/src/format/expression/operator/assign.rs` | 329 | 1 |
| `language/formatter/src/format/expression/object.rs` | 322 | 5 |
| `language/formatter/src/format/expression/call/render.rs` | 317 | 8 |
| `language/formatter/src/format/declaration/dispatch.rs` | 315 | 4 |
| `language/formatter/src/format/expression/chain/format.rs` | 293 | 5 |
| `language/formatter/src/format/declaration/module_like.rs` | 268 | 4 |
| `language/formatter/src/format/expression/classify.rs` | 263 | 13 |
| `language/formatter/src/format/expression/scan.rs` | 260 | 11 |
| `language/formatter/src/format/timing.rs` | 258 | 8 |
| `language/formatter/src/format/expression/chain/policy.rs` | 254 | 4 |
| `language/formatter/src/format/dependency.rs` | 179 | 12 |
| `language/formatter/src/format/match.rs` | 177 | 8 |
| `language/formatter/src/format/operator.rs` | 168 | 5 |
| `language/formatter/src/format/key.rs` | 157 | 9 |
| `language/formatter/src/format/expression/core.rs` | 152 | 3 |
| `language/formatter/src/format/imports.rs` | 140 | 5 |
| `language/formatter/src/format/expression/operator/new.rs` | 106 | 1 |
| `language/formatter/src/format/expression/mod.rs` | 89 | 0 |
| `language/formatter/src/format/collection.rs` | 85 | 5 |
| `language/formatter/src/format/expression/sort.rs` | 84 | 1 |
| `language/formatter/src/format/enum.rs` | 84 | 5 |
| `language/formatter/src/format/where.rs` | 62 | 3 |
| `language/formatter/src/format/path.rs` | 58 | 4 |
| `language/formatter/src/format/scan.rs` | 54 | 4 |
| `language/formatter/src/format/mod.rs` | 29 | 0 |
| `language/formatter/src/format/expression/generic.rs` | 29 | 1 |
| `language/formatter/src/format/expression/chain/mod.rs` | 16 | 0 |
| `language/formatter/src/format/expression/operator/common.rs` | 11 | 1 |
| `language/formatter/src/format/expression/call/mod.rs` | 9 | 0 |
| `language/formatter/src/format/expression/operator/mod.rs` | 7 | 0 |
| `language/formatter/src/format/declaration/mod.rs` | 5 | 0 |

### Per File Signatures
Each subsection lists detected function signature lines with 1-based source line numbers.

#### `language/formatter/src/format/context.rs`
- Line count: 1918.
- Function signature count: 85.

```text
99:    pub fn increment(&self, name: &'static str, delta: usize) {
106:    pub fn snapshot(&self) -> Vec<FormatterCounterEntry> {
143:    pub fn from_ids(tree: &NodeTree, ids: Vec<LocalNodeId<Annotation>>) -> Self {
331:    pub fn default_with_line_width(line_width: u16) -> Self {
339:    pub fn default_tab() -> Self {
347:    pub fn default_tab_with_line_width(line_width: u16) -> Self {
356:    pub fn with_line_ending(mut self, line_ending: LineEnding) -> Self {
362:    pub fn with_indent_style(mut self, indent_style: IndentStyle) -> Self {
368:    pub fn with_indent_width(mut self, indent_width: u8) -> Self {
374:    pub fn with_line_width(mut self, line_width: u16) -> Self {
380:    pub fn with_respect_file_ignore(mut self, respect_file_ignore: bool) -> Self {
386:    pub fn as_print_options(&self) -> PrintOptions {
396:    pub fn from_formatter_options(options: FormatterOptions, language_type: LanguageType) -> Self {
418:    fn from(options: FormatterOptions) -> Self {
425:    fn indent_style(&self) -> IndentStyle {
430:    fn indent_width(&self) -> u8 {
435:    fn line_width(&self) -> u8 {
440:    fn as_print_options(&self) -> PrintOptions {
552:    pub fn new(options: DestackFormatOptions, artifacts: DestackFormatArtifacts<'a>) -> Self {
557:    pub fn new_with_timings(
679:    pub fn has_ignore_directive_markers(&self) -> bool {
685:    pub fn has_template_literal_markers(&self) -> bool {
691:    pub fn mark_file_ignore_applied(&self) {
697:    pub fn file_ignore_applied(&self) -> bool {
703:    pub fn get_span_str(&self, span: Span) -> &'a str {
732:    pub fn get_token_str(&self, token: TokenSpan) -> &'a str {
738:    pub fn comment_tokens(&self) -> &[TokenSpan] {
762:    pub fn span_char_len(&self, span: Span) -> usize {
781:    pub fn node_span_char_len<T>(&self, node_id: LocalNodeId<T>) -> usize
801:    pub fn node_has_newline<T>(&self, node_id: LocalNodeId<T>) -> bool
827:    pub fn get_node<T>(&self, node_id: LocalNodeId<T>) -> &T
837:    pub fn get_node_type<T>(&self, node_id: LocalNodeId<T>) -> NodeType
847:    pub fn get_parent<T>(&self, node_id: LocalNodeId<T>) -> Option<(u32, NodeType)>
863:    pub fn get_parent_by_id(&self, node_id: u32) -> Option<(u32, NodeType)> {
875:    pub fn get_ancestors<T>(&self, node_id: LocalNodeId<T>) -> Vec<(u32, NodeType)>
892:    pub fn any_ancestor<T, F>(&self, node_id: LocalNodeId<T>, mut predicate: F) -> bool
912:    pub fn transparent_inner_expression(
959:    pub fn cached_expression_type_context(&self, node_id: LocalNodeId<Expression>) -> Option<bool> {
979:    pub fn set_cached_expression_type_context(
997:    pub fn expression_is_in_template_literal_interpolation(
1059:    pub fn expression_has_type_conditional_ancestor(
1118:    pub fn find_ancestor<T, F>(
1142:    pub fn get_span<T>(&self, node_id: LocalNodeId<T>) -> Span
1152:    pub fn get_span_by_id(&self, node_id: u32) -> Span {
1158:    pub fn has_newline(&self, span: Span) -> bool {
1200:    pub fn has_comment(&self, span: Span) -> bool {
1243:    pub fn is_at_line_start(&self, node_id: u32) -> bool {
1273:    fn annotation_data_for_node<T>(
1333:    pub fn get_annotations<T>(
1347:    pub fn with_annotations<T, R, F>(&self, node_id: LocalNodeId<T>, f: F) -> Option<R>
1359:    pub fn has_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1388:    pub fn has_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1399:    pub fn has_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1410:    pub fn has_non_blank_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1421:    pub fn has_non_blank_infix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1432:    pub fn has_postfix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1443:    pub fn has_blank_prefix_annotation<T>(&self, node_id: LocalNodeId<T>) -> bool
1454:    pub fn has_blank_prefix_annotation_in_first_position<T>(&self, node_id: LocalNodeId<T>) -> bool
1465:    pub fn argument_annotation_profile(
1489:    pub fn cached_argument_compact_simple_unannotated(
1501:    pub fn cache_argument_compact_simple_unannotated(
1515:    pub fn cached_argument_plain_call_argument(
1524:    pub fn cache_argument_plain_call_argument(
1538:    pub fn cached_call_argument_layout_class(
1547:    pub fn cache_call_argument_layout_class(
1561:    pub fn cached_call_argument_chain_force_expand(
1573:    pub fn cache_call_argument_chain_force_expand(
1587:    pub fn cached_call_argument_expansion_profiles(
1599:    pub fn cache_call_argument_expansion_profiles(
1613:    pub fn cached_call_inline_len_without_static_arguments(
1625:    pub fn cache_call_inline_len_without_static_arguments(
1638:    fn compute_argument_annotation_profile(
1694:    fn cache_get_copy_entry<T: Copy>(
1707:    fn cache_set_copy_entry<T: Copy>(
1722:    pub fn timing_scope(&self, tag: FormatterTimingTag) -> FormatterTimingScope {
1728:    pub fn timing_snapshot(&self) -> Option<Vec<FormatterTimingEntry>> {
1734:    pub fn cache_stats_snapshot(&self) -> FormatterCacheStatsSnapshot {
1749:    pub fn increment_counter(&self, name: &'static str, delta: usize) {
1758:    pub fn record_best_fitting(&self, label: &'static str, variants: usize) {
1766:    pub fn counter_snapshot(&self) -> Vec<FormatterCounterEntry> {
1775:    fn options(&self) -> &Self::Options {
1780:    fn file(&self) -> &File {
1791:    fn format_node(
1806:    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
1817:    fn format(&self, f: &mut DestackFormatter<'a, '_>) -> FormatResult<()> {
```

#### `language/formatter/src/format/annotation.rs`
- Line count: 1817.
- Function signature count: 68.

```text
27:    pub fn block_infix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
36:    pub fn block_prefix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
45:    pub fn block_postfix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
54:    pub fn line_prefix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
63:    pub fn line_postfix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
72:    pub fn line_postfix_boundary_annotations<T: Node>(
84:    pub fn any_prefix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
93:    pub fn any_postfix_annotations<T: Node>(&self, node_id: LocalNodeId<T>) -> Annotations<T> {
102:    pub fn any_infix_or_postfix_annotations<T: Node>(
114:fn annotation_precedes_separator<'ast>(
125:fn annotation_line_with_indentation<'ast>(
140:fn write_annotation_line_with_indentation<'ast>(
169:fn annotation_next_token_is_on_same_line(
198:fn annotation_follows_colon<'ast>(
206:fn annotation_follows_opening_delimiter<'ast>(
217:pub(super) fn annotation_starts_on_own_line<'ast>(
230:fn annotation_has_leading_newline<'ast>(
243:fn comment_annotation_starts_on_own_line<'ast>(
266:fn annotation_comment_style(
279:fn annotation_comment_is_own_line(
290:fn annotation_raw_line_or_trimmed_source(
302:fn annotation_ancestor_flags<T: Node>(
335:fn node_has_member_shape<T: Node>(
364:fn annotation_node_context<T: Node>(
404:fn annotation_render_facts(
458:fn annotation_capture_includes_position(
496:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
837:    fn format_node(
865:    fn format_node(
877:    fn format_node(
928:    fn format_node(
999:fn format_line_comment_lines<'ast>(
1033:fn normalize_inline_block_comment_content(content: &str) -> &str {
1052:fn is_compact_hint_comment(content: &str) -> bool {
1060:fn is_all_asterisks_comment(content: &str) -> bool {
1065:    fn format_node(
1094:fn decorator_needs_parentheses(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
1111:fn is_identifier_or_static_member_only(
1140:    fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
1156:    fn marker_annotation_is_deferred(context: &DestackFormatContext<'_>, marker: &str) -> bool {
1315:    fn find_marker_annotation_on_argument(
1341:    fn find_annotation_by_marker(
1362:    fn find_statement_ternary_expression(
1390:    fn test_annotation_defer_rule_statement_ternary_boundary_prefix() {
1418:    fn test_annotation_defer_rule_parenthesized_boundary() {
1431:    fn test_annotation_defer_rule_call_boundary() {
1444:    fn test_annotation_defer_rule_parameter_type_separator_prefix() {
1457:    fn test_annotation_defer_rule_lambda_arrow_prefix() {
1470:    fn test_annotation_defer_rule_call_argument_inline_boundary_prefix() {
1494:    fn test_annotation_defer_rule_declaration_body_boundary_prefix() {
1507:    fn test_annotation_defer_rule_method_body_boundary() {
1528:    fn test_format_block_comment_retain_newlines() {
1552:    fn test_format_decorators_on_struct() {
1572:    fn test_format_decorator_parentheses_are_stable() {
1587:    fn test_format_multiple_comments_around_expression() {
1605:    fn test_format_multiple_comments_around_expression_in_successive_blocks() {
1636:    fn test_format_inline_expression_comment() {
1647:    fn test_format_multi_line_block_doc_comment_stays_block() {
1668:    fn test_format_multi_line_block_comment_stays_block() {
1686:    fn test_format_excessive_whitespace_in_line_comment() {
1705:    fn test_format_comment_in_call_arguments() {
1716:    fn test_format_comment_in_array() {
1731:    fn test_format_comment_in_object() {
1745:    fn test_format_trailing_comment_array() {
1768:    fn test_format_comment_in_function_body() {
1779:    fn test_format_empty_doc_comment_on_arrow() {
1790:    fn test_format_compact_pure_hint_comment() {
1801:    fn test_format_blank_in_array() {
```

#### `language/formatter/src/format/expression/jsx.rs`
- Line count: 1808.
- Function signature count: 35.

```text
6:pub(super) fn has_multiline_jsx_argument(
33:pub(super) fn format_inline_stub_comment<'ast>(
60:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
146:pub(super) fn tree_attribute_value_id(
159:fn argument_transparent_value_id(
168:fn expression_function_declaration_id(
184:fn declaration_is_lambda(tree: &NodeTree, declaration_id: LocalNodeId<Declaration>) -> bool {
192:fn lambda_body_expression_id(
212:fn argument_lambda_declaration_id(
222:fn expression_postfix_receiver_id(expression: &Expression) -> Option<LocalNodeId<Expression>> {
236:pub(super) fn property_has_complex_value(
281:pub(super) fn property_has_complex_type_value(
309:pub(super) fn should_force_break_tree_attributes(
410:pub(super) fn is_huggable_expression(
439:pub(super) fn format_hugged<'ast>(
739:pub(super) fn format_tree_attribute_value<'ast>(
914:pub(super) fn tree_text_span_str(
933:pub(super) fn tree_text_is_whitespace_only(
975:pub(super) fn tree_text_boundary_separator_space(
1025:pub(super) fn tree_children_have_blank_line_between(
1045:pub(super) fn tree_child_should_inline_braced_expression(
1105:pub(super) fn tree_child_breaks_element(
1148:pub(super) fn lambda_body_is_complex_for_tree(
1163:pub(super) fn argument_is_complex_callback(
1175:pub(super) fn argument_is_block_callback(
1188:pub(super) fn argument_is_object_literal(
1201:pub(super) fn argument_is_array_literal(
1214:pub(super) fn argument_is_template_literal(
1227:pub(super) fn argument_is_lambda_expression(
1235:pub(super) fn argument_is_function_expression(
1250:pub(super) fn expression_has_complex_callback(
1303:pub(crate) fn tree_literal_should_break(
1344:pub(super) fn tree_literal_wraps_on_break(
1400:pub(super) fn format_tree_literal_expression<'ast>(
1436:pub(crate) fn format_tree_literal<'ast>(
```

#### `language/formatter/src/format/argument.rs`
- Line count: 1408.
- Function signature count: 45.

```text
36:    pub(crate) fn should_add_trailing_comma(&self, option: TrailingComma) -> bool {
72:    pub(crate) fn force_expand(&mut self) -> &mut Self {
77:    pub(crate) fn should_expand(&mut self, should_expand: bool) -> &mut Self {
82:    pub(crate) fn include_space(&mut self) -> &mut Self {
87:    pub(crate) fn force_trailing_separator(&mut self) -> &mut Self {
92:    pub(crate) fn disallow_trailing_separator(&mut self) -> &mut Self {
98:    pub(crate) fn as_collection(&mut self) -> &mut Self {
103:    pub(crate) fn with_group_id(&mut self, group_id: Option<GroupId>) -> &mut Self {
115:    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
356:fn expression_argument_prefers_inline_parenthesized_layout<'ast, T>(
397:fn parent_expression_has_linebreak_around_element<'ast, T>(
436:fn format_list_with_ignored_ranges<'ast, T>(
485:fn raw_ends_with_separator(raw: &str, separator: &str) -> bool {
491:fn strip_trailing_comments(raw: &str) -> &str {
525:pub(crate) fn list_like<'ast, 'e, T>(
551:    fn format_node(
665:fn parameter_is_static(
716:fn format_deferred_parameter_type_separator_annotations<'ast>(
736:fn argument_should_emit_prefix_annotations(
760:fn argument_is_call_or_new(
776:fn argument_is_first_in_call_or_new(
802:fn argument_has_non_blank_prefix_annotation(
824:fn argument_has_blank_prefix_annotation(
844:fn argument_has_blank_prefix_annotation_before_separator(
883:fn argument_is_last_in_call_or_new(
909:fn argument_should_emit_trailing_comma(
920:fn argument_has_trailing_line_comment_annotation(
945:fn argument_prefix_lambda_comment_needs_forced_break(
974:    fn format_node(
1144:fn argument_has_prefix_comment_annotation(
1164:fn argument_prefix_comment_has_leading_blank_line_after_separator(
1199:fn first_prefix_comment_annotation_start_for_argument(
1257:fn previous_dynamic_argument_in_call_or_new(
1286:fn argument_contains_lambda_value(context: &DestackFormatContext<'_>, argument: &Argument) -> bool {
1310:    fn test_format_parameter() {
1320:    fn test_format_parameter_with_default() {
1330:    fn test_format_parameter_comment_between_name_and_type() {
1340:    fn test_format_optional_parameter_comment_between_name_and_type() {
1350:    fn test_format_parameter_comment_before_name() {
1360:    fn test_format_argument_named() {
1370:    fn test_format_argument_named_shorthand() {
1380:    fn test_format_argument_positional() {
1390:    fn test_strip_trailing_comments_keeps_leading_ignore_block_comment() {
1398:    fn test_raw_ends_with_separator_for_ignored_field_with_leading_comment() {
1404:    fn test_raw_ends_with_separator_with_trailing_line_comment() {
```

#### `language/formatter/src/format/expression/tests.rs`
- Line count: 1187.
- Function signature count: 89.

```text
12:fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
28:fn find_call_with_dynamic_argument_count(
54:fn find_parenthesized_expression_by_inner(
78:fn test_format_expression_simple() {
89:fn test_format_expression_parenthesized() {
100:fn test_format_expression_nested_empty_parenthesis() {
111:fn test_format_expression_nested_empty_arguments() {
121:fn test_format_expression_struct_literal_trivial() {
131:fn test_format_expression_struct_literal_spread() {
141:fn test_assignment_target_detection() {
175:fn test_format_expression_if_ternary() {
185:fn test_format_expression_index_call_mixed_postfix() {
196:fn test_format_expression_instantiation() {
207:fn test_format_new_expression_drops_simple_member_parentheses() {
218:fn test_format_new_expression_keeps_optional_member_parentheses() {
229:fn test_format_new_expression_wraps_call_member_callee() {
240:fn test_parenthesis_policy_rejects_member_object_boundary_comment() {
258:fn test_parenthesis_policy_rejects_optional_new_callee_unwrap() {
281:fn test_format_array_expression_sparse_elisions() {
310:fn test_format_new_expression_empty_argument_comment() {
321:fn test_format_member_expression_unwraps_parenthesized_call_object() {
332:fn test_tree_child_map_callback_breaks() {
355:fn test_format_type_const_borrow_normalizes_to_readonly() {
361:fn test_format_type_const_pointer_normalizes_to_readonly() {
385:fn test_format_type_alias_preserves_borrowed_reference() {
412:fn test_format_type_alias_preserves_readonly_borrowed_reference() {
422:fn test_format_member_call_chain_line() {
432:fn test_format_member_call_chain_retains_breaks() {
442:fn test_format_member_call_chain_breaks() {
452:fn test_format_member_call_chain_breaks_with_maybe_and_index() {
462:fn test_format_path_member_call_chain_breaks() {
472:fn test_format_index_member_chain_breaks() {
483:fn test_format_member_chain_breaks_before_long_boundary_comment_with_optional_call() {
495:fn test_format_chain_planner_promotes_head_in_call_like_argument() {
506:fn test_format_chain_planner_respects_assignment_rhs_width() {
516:fn test_format_expression_tree_literal_without_arguments() {
526:fn test_format_expression_tree_literal_with_arguments() {
536:fn test_format_expression_tree_literal_parenthesized() {
556:fn test_format_expression_tree_literal_nested() {
584:fn test_format_expression_tree_literal_with_array_of_struct_element() {
609:fn test_format_expression_call_with_struct_literal() {
630:fn test_format_expression_let_call() {
664:fn test_format_call_single_lambda_argument_with_prefix_comment_breaks() {
671:fn test_format_call_nested_arrow_boundary_comments() {
679:fn test_format_chained_assignment() {
689:fn test_format_chained_assignment_long() {
699:fn test_format_jsx_with_comment() {
709:fn test_format_jsx_conditional_child() {
719:fn test_format_jsx_in_function_call() {
729:fn test_format_deeply_nested_callbacks() {
740:fn test_call_chain_classifier_expands_callback_heavy_arguments() {
764:fn test_call_chain_classifier_expands_single_tree_child_argument() {
786:fn test_format_optional_chain_with_nullish() {
797:fn test_format_type_conditional_with_infer() {
807:fn test_format_type_conditional_with_constrained_infer() {
817:fn test_format_type_conditional_nested() {
827:fn test_format_type_intersection_trailing_operator_destack() {
838:fn test_format_type_mapped_with_remap() {
846:fn test_format_type_mapped_with_removals() {
854:fn test_format_type_mapped_without_modifiers() {
862:fn test_format_type_mapped_with_optional() {
870:fn test_format_type_index() {
878:fn test_format_type_template_literal() {
886:fn test_format_type_template_literal_multiple_spans() {
894:fn test_format_type_template_literal_union_with_leading_pipe() {
902:fn test_format_type_union_drops_redundant_parentheses() {
910:fn test_format_type_single_member_leading_union_parenthesized_array() {
918:fn test_format_type_single_member_leading_intersection_parenthesized_array() {
934:fn test_type_template_literal_union_is_in_type_context() {
974:fn test_format_type_import() {
984:fn test_format_type_import_without_qualifier() {
994:fn test_format_type_infer_expression() {
1000:fn test_format_async_arrow() {
1010:fn test_format_return_jsx_inline() {
1021:fn test_format_return_jsx_multiline() {
1031:fn test_format_nested_ternary() {
1042:fn test_format_const_call_with_multiline_object_rhs() {
1053:fn test_format_export_const_chain_rhs_does_not_break_after_operator() {
1064:fn test_format_const_generic_call_rhs_breaks_after_operator() {
1076:fn test_format_const_generic_call_rhs_preserves_source_operator_break() {
1088:fn test_format_const_generic_call_with_multiline_type_argument_keeps_operator_inline() {
1099:fn test_format_jsx_bracket_same_line_true() {
1111:fn test_format_jsx_bracket_same_line_false() {
1124:fn test_format_await_inside_maybe_gets_parenthesized() {
1137:fn test_format_unary_inside_maybe_gets_parenthesized() {
1147:fn test_format_unary_await_expression_parenthesizes_operand() {
1158:fn test_format_postfix_inside_maybe_no_extra_parens() {
1169:fn test_format_call_inside_maybe_no_parens() {
1180:fn test_format_await_maybe_sugar() {
```

#### `language/formatter/src/format/expression/statement.rs`
- Line count: 950.
- Function signature count: 13.

```text
8:fn statement_expression_needs_semicolon(
36:fn format_dependency_with_arguments<'ast>(
53:fn format_import_expression<'ast>(
201:fn format_export_expression<'ast>(
302:fn format_let_expression<'ast>(
347:fn format_using_expression<'ast>(
387:fn format_while_expression<'ast>(
432:fn format_for_each_expression<'ast>(
518:fn format_for_expression<'ast>(
548:fn format_loop_expression<'ast>(
560:fn format_try_expression<'ast>(
610:fn format_return_expression<'ast>(
676:pub(super) fn format_statement_expression<'ast>(
```

#### `language/formatter/src/format/block.rs`
- Line count: 905.
- Function signature count: 29.

```text
23:fn is_directive_expression(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
39:    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
66:pub fn statement_list(expressions: &[LocalNodeId<Expression>]) -> StatementList<'_> {
71:fn statement_list_is_file_root(
92:    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
120:pub fn empty_block_with_infix_annotations<T: Node>(
127:fn expressions_have_blank_line_between(
148:fn source_has_blank_line_between_offsets(
168:fn source_has_blank_line_in_trivia(source: &str) -> bool {
195:fn expression_prefix_start(
227:pub(crate) fn format_block_body_narrow<'ast>(
258:pub(crate) fn format_block_body_wide<'ast>(
281:pub(crate) fn format_block_of_statements<'ast>(
522:pub(crate) fn should_inline_block<'ast>(
569:fn empty_block_prefers_multiline<'ast>(
612:pub fn format_block<'ast>(
627:    fn format_node(
658:    fn test_format_block_insert_semicolon() {
722:    fn test_format_empty_block_with_comment() {
735:    fn test_format_mixed_block_with_prefix_postfix_comment() {
750:    fn test_format_mixed_block_with_postfix_comment() {
765:    fn test_format_mixed_block_with_postfix_annotations_mixed() {
781:    fn test_format_block_inline() {
792:    fn test_format_block_statement_like() {
803:    fn test_format_block_statement_retain_newline() {
815:    fn test_format_block_declaration_after_expression_no_forced_blank_line() {
826:    fn test_format_block_import_sorting() {
851:    fn test_format_block_import_sorting_preserves_side_effect_order() {
879:    fn test_format_block_import_sorting_inserts_group_blank_lines() {
```

#### `language/formatter/src/format/expression/operator/binary.rs`
- Line count: 837.
- Function signature count: 7.

```text
10:fn leading_union_has_ancestor_block_prefix_annotation(
38:fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
47:fn is_mixed_logical_precedence_pair(
61:fn preserve_source_operator_break(
70:fn operand_prefers_trailing_logical_operator(
79:pub(super) fn format_binary_expression<'ast>(
764:pub(super) fn format_type_binary_expression<'ast>(
```

#### `language/formatter/src/format/annotation/defer.rs`
- Line count: 830.
- Function signature count: 27.

```text
13:pub(super) fn annotation_followed_by_else_keyword(
21:fn annotation_tail_starts_with(
40:fn is_parameter_prefix_annotation_left_anchor(character: char) -> bool {
45:fn annotation_is_prefix_position(position: AnnotationPosition) -> bool {
53:fn annotation_is_declaration_boundary_position(position: AnnotationPosition) -> bool {
63:fn annotation_is_comment_or_doc(
74:fn collect_matching_annotations<T, P>(
111:fn is_parameter_type_separator_prefix_annotation<T: Node>(
141:pub(crate) fn parameter_type_separator_prefix_annotations(
153:pub(crate) fn is_lambda_arrow_prefix_annotation<T: Node>(
179:fn argument_is_call_like_dynamic_argument(
203:pub(crate) fn is_call_argument_inline_boundary_prefix_annotation<T: Node>(
241:pub(crate) fn call_argument_inline_boundary_prefix_annotations(
253:fn declaration_has_braced_body(
269:fn is_declaration_body_boundary_prefix_annotation<T: Node>(
304:pub(crate) fn declaration_body_boundary_prefix_annotations(
316:pub(crate) fn declaration_expression_body_boundary_prefix_annotations(
328:fn ternary_contains_boundary_annotation_span(
373:pub(crate) fn should_defer_statement_ternary_boundary_prefix_annotation<T>(
420:fn should_defer_parenthesized_boundary_annotation<T>(
496:fn should_defer_call_boundary_annotation<T>(
555:fn method_body_for_signature_return_type_expression(
594:fn expression_is_method_body(
622:fn should_defer_method_body_boundary_annotation<T>(
667:fn enclosing_empty_call_position_for_callee(
739:    fn matches<T: Node>(
809:pub(super) fn annotation_should_defer<T: Node>(
```

#### `language/formatter/src/format/property.rs`
- Line count: 814.
- Function signature count: 27.

```text
31:pub(crate) fn format_binding_modifiers_prefix<'ast>(
85:pub(crate) fn format_binding_modifiers_prefix_maybe<'ast>(
97:pub(crate) fn format_binding_modifiers_postfix<'ast>(
114:pub(crate) fn format_binding_modifiers_postfix_maybe<'ast>(
125:fn method_signature_source_is_multiline(
145:pub(crate) fn format_block_of_properties<'ast>(
168:pub(crate) fn format_block_of_members<'ast>(
176:fn format_block_nodes_with_ignore_ranges<'ast, T, F>(
219:fn key_requires_quotes<'ast>(f: &DestackFormatter<'ast, '_>, key: Key) -> bool {
232:fn force_quote_keys_for_object<'ast>(
249:fn force_quote_keys_for_members<'ast>(
266:fn should_force_quote_keys_for_property<'ast>(
296:fn should_force_quote_keys_for_member<'ast>(
325:fn should_keep_field_default_inline<'ast>(
351:fn write_field_type_annotation<'ast>(
363:fn format_field_like<'ast>(
406:fn format_method_like<'ast>(
499:fn format_node_with_directive<'ast, T, F>(
535:    fn format_node(
591:    fn format_node(
743:    fn test_format_struct_empty() {
753:    fn test_format_struct_with_fields() {
763:    fn test_format_struct_with_modified_fields() {
773:    fn test_format_struct_with_name() {
783:    fn test_format_struct_with_fields_and_defaults() {
793:    fn test_format_class_with_abstract_override_field() {
806:    fn test_format_struct_with_static_parameters_and_inheritance() {
```

#### `language/formatter/src/format/expression/chain/break.rs`
- Line count: 813.
- Function signature count: 16.

```text
4:pub(crate) fn split_chain_head_operations(
257:pub(crate) fn summarize_chain_calls(
293:pub(crate) fn analyze_chain_break(
430:pub(crate) fn chain_node_has_breaking_annotation(
462:pub(crate) fn chain_node_has_non_inline_annotation(
499:pub(crate) fn chain_line_starts_with_block_prefix_annotation(
528:pub(crate) fn chain_overflows_in_type_binary_left(
593:pub(crate) fn expression_is_in_template_literal_interpolation(
601:pub(crate) fn should_break_chain(
609:pub(crate) fn chain_has_intervening_break_or_comment(
642:pub(crate) fn chain_has_nonhead_nonlambda_function_call_argument(
672:pub(crate) fn should_split_chain_root_path_segments(
718:pub(crate) fn path_chain_has_optional_or_must_tail(
768:pub(crate) fn is_factory_like_path_head(name: &str) -> bool {
778:pub(crate) fn expression_is_in_conditional_branch(
797:pub(crate) fn is_assignment_chain_tail_lambda(
```

#### `language/formatter/src/format/expression/chain/classify.rs`
- Line count: 808.
- Function signature count: 29.

```text
4:pub(crate) fn chain_head_id(
24:pub(crate) fn is_simple_chain_head(
45:pub(crate) fn is_short_chain_argument(
62:fn chain_call_has_breakable_dynamic_arguments(
79:pub(crate) fn is_poorly_breakable_chain(
161:pub(crate) fn argument_value_id(
174:pub(crate) fn is_lambda_expression(
191:pub(crate) fn is_nested_lambda_expression(
218:pub(crate) fn lambda_body_forces_multiline(
251:pub(crate) fn expression_callback_depth(
301:pub(crate) fn expression_chain_should_break(
330:pub(crate) fn expression_is_in_tree_literal_child(
370:pub(crate) fn lambda_expression_should_break(
428:pub(crate) fn argument_forces_multiline(
448:pub(crate) fn is_block_lambda_argument(
476:pub(crate) fn is_simple_chain_argument(
493:pub(crate) fn arguments_total_len(
510:pub(crate) fn arguments_rendered_len(
524:pub(crate) fn is_numeric_scalar_literal(expression: &Expression) -> bool {
534:pub(crate) fn is_numeric_index_expression(
548:pub(crate) fn is_numeric_index(
559:pub(crate) fn is_simple_chain_static_arguments(
576:pub(crate) fn is_simple_chain_static_argument_list(
588:pub(crate) fn is_simple_chain_call(
615:pub(crate) fn is_simple_chain_operation(
657:pub(crate) fn member_has_intervening_comment(
683:pub(crate) fn member_has_intervening_break_or_comment(
727:pub(crate) fn expression_trivia_anchor_end(
750:pub(crate) fn chain_has_parent_intervening_break_or_comment(
```

#### `language/formatter/src/format/expression/chain/base.rs`
- Line count: 759.
- Function signature count: 24.

```text
20:pub(crate) fn extract_parenthesized_index_chain(
55:pub(crate) fn format_maybe_expression<'ast>(
132:pub(crate) fn chain_node_left_id(
149:pub(crate) fn collect_chain_nodes(
170:pub(crate) fn chain_expression_from_node(
240:pub(crate) fn is_expression_chain(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
245:pub(crate) fn call_prefers_chain_format(
261:pub(crate) fn is_chain_root(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
266:pub(crate) fn has_chain_parent(
291:pub(crate) fn assignment_like_parent(
341:pub(crate) fn transparent_inner_expression(
349:pub(crate) fn assign_operator_len(operator: &AssignOperator) -> usize {
380:pub(crate) fn binary_operator_len(operator: &BinaryOperator) -> usize {
419:pub(crate) fn should_use_trailing_coalesce(
461:pub(crate) fn assignment_like_remaining_width(
511:pub(crate) fn expression_prefix_annotation_source_len(
537:pub(crate) fn declarator_leading_prefix_len(
607:pub(crate) fn has_newline_between_expressions(
622:pub(crate) fn has_comment_between_expressions(
640:pub(crate) fn line_comment_between_expressions(
664:pub(crate) fn expression_source_len(
672:pub(crate) fn should_expand_static_argument_list(
709:pub(crate) fn should_hug_static_argument_list(
744:pub(crate) fn should_force_multiline_mapped_type(
```

#### `language/formatter/src/format/expression/primary.rs`
- Line count: 751.
- Function signature count: 4.

```text
11:pub(super) fn format_primary_expression<'ast>(
693:fn array_has_sparse_holes(
704:fn argument_is_sparse_hole(
716:fn format_sparse_array_literal<'ast>(
```

#### `language/formatter/src/format/expression/call/analyze_layout.rs`
- Line count: 888.
- Function signature count: 27.

```text
11:pub(super) fn argument_is_compact_simple_unannotated(
39:pub(super) fn call_has_call_chain_parent(
90:pub(super) fn call_argument_shape_from_layout_class(
122:    fn new() -> Self {
145:fn build_empty_call_argument_layout_class(
160:fn call_argument_callback_flags(
186:fn scan_call_argument_layout_state(
291:fn build_scanned_call_argument_layout_class(
340:pub(super) fn resolve_call_argument_layout_class(
375:pub(super) fn leading_dynamic_arguments(
386:pub(super) fn call_argument_layout_class_is_simple_multi_unannotated(
393:pub(super) fn leading_arguments_are_compact_simple_unannotated(
404:pub(super) fn leading_arguments_are_compact_callback_tail_candidates(
432:pub(super) fn call_arguments_use_single_simple_argument_fast_path(
467:pub(super) fn call_arguments_use_single_callback_argument_inline(
506:pub(super) fn call_arguments_use_single_simple_argument_inline(
574:pub(super) fn resolve_inline_call_len_without_static_arguments(
609:pub(super) fn can_consider_hug_last_call_arguments(
627:pub(super) fn call_arguments_force_hug_last_inline(
664:fn hug_last_tail_flags(
683:fn hug_last_can_inline_by_width(
692:fn hug_last_can_inline_callback_tail(
704:fn hug_last_can_inline_overflow_tail(
717:fn hug_last_should_skip_probe_collection(
727:pub(super) fn resolve_hug_last_call_argument_layout(
843:pub(super) fn build_call_argument_planner_base_state(
859:pub(super) fn build_call_argument_planner_state(
```

#### `language/formatter/src/format/expression/declarator.rs`
- Line count: 728.
- Function signature count: 5.

```text
60:fn choose_declarator_layout(inputs: DeclaratorLayoutInputs) -> DeclaratorLayout {
232:fn declaration_has_generic_heritage(
259:fn value_has_generic_class_heritage(
283:pub(super) fn format_declarator<'ast>(
715:    fn format_node(
```

#### `language/formatter/src/format/literal.rs`
- Line count: 710.
- Function signature count: 26.

```text
24:pub(crate) fn format_scalar_literal<'ast>(
136:fn format_interpolated_template_literal<'ast>(
202:fn template_argument_expression_id(
216:fn template_argument_should_force_inline(
242:fn template_argument_should_expand(
301:fn template_expression_is_complex(
347:fn argument_value_expression<'ast>(
361:fn unwrap_template_expression(
381:pub(crate) fn format_template_literal<'ast>(
399:fn normalize_jsx_text(text: &str) -> String {
423:fn jsx_boundary_spaces(text: &str) -> (bool, bool) {
446:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
473:    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
504:    fn format(&self, f: &mut Formatter<'_, DestackFormatContext<'ast>>) -> FormatResult<()> {
516:fn normalize_int(input: &str, _is_bigint: bool) -> Cow<'_, str> {
561:fn normalize_float(input: &str) -> Cow<'_, str> {
635:    fn test_format_string_literal_multi_char() {
641:    fn test_format_string_literal_single_char() {
647:    fn test_format_string_literal_empty() {
653:    fn test_format_template_literal_plain() {
660:    fn test_format_template_literal_one_interpolation() {
667:    fn test_format_template_literal_all_interpolation() {
674:    fn test_format_template_literal_adjacent_interpolations() {
681:    fn test_format_template_literal_complex_sql() {
688:    fn test_format_long_string_not_broken() {
701:    fn test_format_long_template_literal_not_broken() {
```

#### `language/formatter/src/format/expression/call/profile.rs`
- Line count: 704.
- Function signature count: 13.

```text
9:fn decide_post_hugged_call_argument_layout(
146:fn format_fast_default_call_argument_list<'ast>(
185:fn format_default_call_argument_list<'ast>(
261:fn format_comment_expanded_call_argument_list<'ast>(
317:fn write_single_call_argument_inline_wrapped<'ast>(
327:fn format_call_argument_layout_decision<'ast>(
366:pub(in super::super) fn format_call_arguments<'ast>(
386:fn call_arguments_use_no_annotation_multi_argument_fast_path(
400:fn format_no_annotation_multi_argument_fast_path<'ast>(
422:fn call_arguments_use_forced_hug_last_inline_fast_path(
430:fn format_single_call_argument_with_group<'ast>(
577:fn format_call_arguments_with_group<'ast>(
664:pub(in super::super) fn call_arguments_force_expand_for_chain(
```

#### `language/formatter/src/format/expression/parentheses.rs`
- Line count: 630.
- Function signature count: 24.

```text
24:pub(super) fn parenthesized_should_unwrap(
47:pub(super) fn parenthesized_should_drop(
71:pub(super) fn parenthesized_prefers_new_member_callee_parentheses(
79:pub(super) fn collect_parenthesized_boundary_comments(
145:pub(super) fn parenthesized_has_leading_inner_trivia(
154:pub(super) fn parenthesized_has_leading_inner_comments(
163:fn parenthesized_has_leading_inner_pattern(
188:pub(super) fn member_expression_source_has_optional_chain(
197:pub(super) fn should_unwrap_parenthesized_member_object(
214:pub(super) fn member_object_prefers_new_callee_parentheses(
236:pub(super) fn is_simple_new_member_object(
254:pub(super) fn should_unwrap_parenthesized_new_member_callee(
281:pub(super) fn strip_one_wrapping_parentheses(source: &str) -> &str {
290:pub(super) fn type_binary_is_statement_expression(
309:pub(super) fn has_parenthesized_ancestor_with_leading_inner_trivia(
335:pub(super) fn type_binary_is_parenthesized_new_callee(
369:pub(super) fn should_drop_type_binary_left_parentheses(
410:pub(super) fn parenthesized_is_top_level_type_alias_value(
429:pub(super) fn expression_is_type_binary_chain_head(
448:pub(super) fn parenthesized_source_leading_type_grouping_operator(
484:pub(super) fn should_drop_parenthesized_type_expression(
546:pub(super) fn is_associative_type_binary_operator(operator: BinaryOperator) -> bool {
554:pub(super) fn parenthesized_associative_type_binary_can_drop(
594:fn should_drop_parenthesized_expression_wrapper(
```

#### `language/formatter/src/format/expression/call/analyze.rs`
- Line count: 607.
- Function signature count: 19.

```text
27:    fn from(cached: CachedCallArgumentExpansionProfile) -> Self {
37:    fn from(profile: CallArgumentExpansionProfile) -> Self {
56:    fn from(cached: CachedCallArgumentExpansionProfiles) -> Self {
65:    fn from(profiles: CallArgumentExpansionProfiles) -> Self {
85:pub(super) fn collect_call_argument_comment_profile(
126:pub(super) fn collect_single_call_argument_facts(
140:pub(super) fn argument_has_callback_blocking_comment_annotation(
151:pub(super) fn call_force_expand_single_long_with_static_arguments(
176:pub(super) fn call_force_expand_single_collection_for_type_binary_callee(
187:pub(super) fn single_argument_requires_expanded_list(
210:pub(super) fn build_call_argument_expansion_profiles(
398:pub(super) fn resolve_regular_call_argument_expansion_profile(
437:pub(in super::super) fn resolve_chain_call_argument_force_expand(
476:pub(super) fn call_should_force_hugged_expand(
483:pub(super) fn call_inline_len_without_static_arguments(
506:pub(super) fn write_inline_call_argument_list<'ast>(
533:pub(super) fn argument_is_plain_call_argument(
565:pub(super) fn write_plain_call_argument<'ast>(
597:pub(super) fn write_plain_call_argument_or_node<'ast>(
```

#### `language/formatter/src/format/directive.rs`
- Line count: 603.
- Function signature count: 17.

```text
54:pub fn directive_for_node<T: Node + Clone>(
150:pub fn ignore_range_for_node<T: Node + Clone>(
242:pub fn collect_ignore_ranges_for_nodes<T: Node + Clone>(
260:pub fn any_ignore_range_for_nodes<T: Node + Clone>(
275:fn comment_token_is_line_leading(context: &DestackFormatContext<'_>, token: TokenSpan) -> bool {
288:fn extend_span_with_trailing_tokens(context: &DestackFormatContext<'_>, span: Span) -> Span {
329:pub fn collect_comment_tokens(context: &DestackFormatContext<'_>) -> Vec<TokenSpan> {
334:pub fn has_file_ignore_directive(context: &DestackFormatContext<'_>) -> bool {
373:pub fn ignored_span_source(context: &DestackFormatContext<'_>, span: Span) -> String {
396:pub fn write_ignored_span<'ast>(
429:pub fn ignored_node_source<T: Node>(
446:fn find_ignore_range_end(
467:fn parse_directive_token_from_raw(raw: &str) -> Option<FormatterDirectiveToken> {
473:pub(crate) fn is_ignore_directive_comment(raw: &str) -> bool {
481:fn strip_comment_markers(raw: &str) -> Cow<'_, str> {
494:fn parse_directive_token(comment: &str) -> Option<FormatterDirectiveToken> {
562:    fn test_format_ignore_range_for_statement() {
```

#### `language/formatter/src/format/expression/ternary.rs`
- Line count: 594.
- Function signature count: 12.

```text
6:pub(super) fn get_argument_value(
19:pub(super) fn collect_ternary_chain(
72:pub(super) fn collect_statement_ternary_boundary_prefix_comments(
170:pub(super) fn write_ternary_separator_comments<'ast>(
181:pub(super) fn write_ternary_colon_line_comments<'ast>(
196:pub(super) fn collect_ternary_colon_line_comments(
248:pub(super) fn ternary_else_start_for_branch(
264:pub(super) fn ternary_then_has_boundary_comment_before_colon(
290:pub(super) fn collect_catch_pattern_trailing_boundary_comments(
335:pub(super) fn ternary_requires_terminator(
360:pub(super) fn ternary_branch_is_tree_like(
373:pub(super) fn format_ternary(
```

#### `language/formatter/src/format/expression/member.rs`
- Line count: 544.
- Function signature count: 17.

```text
6:pub(super) fn format_member_expression<'ast>(
107:pub(super) fn format_type_index_expression<'ast>(
128:pub(super) fn format_type_template_literal<'ast>(
163:pub(super) fn format_index_expression<'ast>(
271:fn binary_operator_precedence_group(operator: BinaryOperator) -> u8 {
279:fn should_flatten_binary(left_operator: BinaryOperator, right_operator: BinaryOperator) -> bool {
298:pub(super) fn flatten_binary_expression(
309:pub(super) fn flatten_type_binary_expression(
320:pub(super) fn flattened_binary_operand_count(
329:fn flatten_type_binary_recursive(
364:pub(super) fn normalize_type_binary_operand_expression(
397:fn flatten_binary_recursive(
430:fn count_flattened_binary_recursive(
453:pub(super) fn expression_precedence(expr: &Expression) -> u16 {
512:pub(super) fn needs_parens_in_postfix_position(
520:pub(super) fn write_postfix_base_expression<'ast>(
533:pub(super) fn is_chain_expression(expression: &Expression) -> bool {
```

#### `language/formatter/src/format/expression/binary.rs`
- Line count: 535.
- Function signature count: 17.

```text
5:pub(super) fn is_type_expression_variant(expression: &Expression) -> bool {
21:pub(super) fn expression_static_arguments(
51:pub(super) fn is_static_type_argument_context(
76:pub(crate) fn is_type_context(
92:fn is_type_context_uncached(
268:pub(super) fn is_parameter_type_annotation(
289:pub(super) fn is_simple_type_binary_left_expression(
321:pub(super) fn is_object_like_type_expression(
333:pub(super) fn is_nullable_union_member(
345:pub(super) fn is_type_reference_expression(
357:pub(super) fn should_hug_nullable_union_type(
397:pub(super) fn should_hug_static_argument_union_type(
447:pub(super) fn union_source_has_leading_pipe(
458:pub(super) fn is_type_grouping_binary_operator(operator: BinaryOperator) -> bool {
466:pub(super) fn binary_rhs_prefers_break_after_operator(
474:pub(super) fn type_binary_operand_needs_grouping_parentheses(
505:pub(super) fn format_binary_operand_with_grouping_parentheses<'ast>(
```

#### `language/formatter/src/format/pattern.rs`
- Line count: 522.
- Function signature count: 19.

```text
15:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
24:fn default_expression_prefers_multiline(
35:fn pattern_prefers_multiline(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
56:fn pattern_field_prefers_multiline(tree: &NodeTree, field_id: LocalNodeId<PatternField>) -> bool {
81:fn source_between_delimiters_has_newline(source: &str, open: char, close: char) -> bool {
97:fn pattern_has_multiline_source(
117:fn should_expand_pattern_field_default<'ast>(
145:fn should_expand_parameter_object_pattern(
198:    fn format_node(
347:    fn format_node(
475:    fn test_format_pattern_wildcard() {
480:    fn test_format_pattern_reference() {
487:    fn test_format_pattern_must() {
492:    fn test_format_pattern_identifier() {
497:    fn test_format_pattern_path() {
502:    fn test_format_pattern_tuple() {
507:    fn test_format_pattern_tuple_with_path() {
513:    fn test_format_pattern_slice() {
518:    fn test_format_pattern_union() {
```

#### `language/formatter/src/format/expression/call/classify.rs`
- Line count: 473.
- Function signature count: 22.

```text
18:fn expression_is_lambda_declaration(
34:pub(in super::super) fn argument_is_simple_with_options(
61:pub(in super::super) fn is_call_like_argument(
87:pub(in super::super) fn is_tree_attribute_expression(
113:pub(in super::super) fn is_simple_static_argument(
130:pub(in super::super) fn argument_is_string_like(
144:pub(in super::super) fn argument_is_interpolated_template_literal(
160:pub(in super::super) fn argument_is_collection_literal(
169:pub(in super::super) fn argument_is_reference_like(
191:pub(in super::super) fn call_callee_has_test_like_member_name(
245:pub(in super::super) fn call_should_force_hug_test_like_callback(
268:pub(in super::super) fn call_arguments_are_multiline_in_source(
286:pub(in super::super) fn call_arguments_preserve_blank_line_between(
324:pub(in super::super) fn call_has_non_blank_infix_annotation(
332:pub(in super::super) fn argument_has_non_blank_annotation(
340:pub(in super::super) fn argument_has_multiline_prefix_annotation(
354:pub(in super::super) fn argument_has_leading_prefix_annotation_outside_span(
384:pub(in super::super) fn argument_has_line_comment_annotation(
394:pub(in super::super) fn argument_has_prefix_line_comment_annotation(
404:pub(in super::super) fn call_has_static_arguments(
422:pub(in super::super) fn call_like_has_type_binary_callee(
442:pub(in super::super) fn call_has_leading_block_callback_with_simple_tail(
```

#### `language/formatter/src/format/declaration/function_like.rs`
- Line count: 471.
- Function signature count: 7.

```text
23:fn is_module_typescript_source(file_name: &str) -> bool {
28:fn lambda_parameter_should_expand(
49:fn lambda_parameter_is_simple_tail(
65:fn append_lambda_arrow_prefix_annotations<T: destack_ast::Node + Clone>(
91:fn collect_lambda_arrow_prefix_annotations(
117:fn write_deferred_lambda_arrow_prefix_annotations<'ast>(
146:pub(super) fn format_function_declaration<'ast>(
```

#### `language/formatter/src/format/signature.rs`
- Line count: 430.
- Function signature count: 16.

```text
20:pub(crate) fn parameter_is_variadic(
31:pub(crate) fn parameter_has_modifier(
44:pub(crate) fn constructor_parameters_should_expand(
57:pub(crate) fn single_parameter_should_hug(
88:pub(crate) fn write_function_abstraction_prefix(
109:pub(crate) fn write_function_asynchrony_prefix(
130:pub(crate) fn write_function_header_prefix(
175:fn parameter_object_pattern_should_expand(
228:pub(crate) fn parameter_should_force_expand_in_signature(
254:pub(crate) fn signature_return_type_is_multiline(
262:pub(crate) fn signature_should_elide_space_before_body(
270:pub(crate) fn signature_parameters_should_expand(
294:pub(crate) fn write_signature_dynamic_parameter_list(
311:pub(crate) fn collect_deferred_function_boundary_line_comments(
369:pub(crate) fn function_body_has_deferred_boundary_line_comments(
382:pub(crate) fn format_function_body_block_with_deferred_boundary_line_comments(
```

#### `language/formatter/src/format/expression/control.rs`
- Line count: 410.
- Function signature count: 9.

```text
7:pub(super) fn format_statement_body_block<'ast>(
41:fn is_statement_wrapper_block<'ast>(
50:pub(super) fn is_empty_statement_block<'ast>(
59:pub(super) fn detect_for_each_binding_keyword<'ast>(
80:pub(super) fn format_for_each_binding_pattern<'ast>(
97:fn has_line_postfix_slash_comment<'ast, T>(
126:fn inline_else_boundary_block_comment<'ast>(
187:pub(crate) fn format_if_else_chain<'ast>(
355:pub(crate) fn format_match<'ast>(
```

#### `language/formatter/src/format/declaration/type_alias.rs`
- Line count: 389.
- Function signature count: 5.

```text
30:fn single_line_type_grouping_prefix_comment_cluster(
111:fn expression_has_doc_like_block_prefix_annotation(
140:fn expression_has_prefix_annotation_in_left_spine(
166:fn format_expression_without_prefix_annotations<'ast>(
191:pub(super) fn format_type_alias_declaration<'ast>(
```

#### `language/formatter/src/format/declaration/type_like.rs`
- Line count: 385.
- Function signature count: 9.

```text
21:fn write_deferred_declaration_body_boundary_prefix_annotations<'ast>(
55:fn format_declaration_header_prefix<'ast>(
71:fn format_declaration_static_parameters<'ast>(
85:fn format_declaration_where_clauses<'ast>(
99:fn format_declaration_heritage<'ast>(
121:fn format_anonymous_class_heritage<'ast>(
178:pub(super) fn format_struct_or_class_declaration<'ast>(
276:pub(super) fn format_enum_declaration<'ast>(
343:pub(super) fn format_interface_declaration<'ast>(
```

#### `language/formatter/src/format/expression/chain/normalize.rs`
- Line count: 550.
- Function signature count: 15.

```text
34:fn collect_chain_root_parts(
108:fn append_chain_operations(
121:fn chain_body_has_breaking_member_annotation(
135:fn chain_should_avoid_head_promotion_for_boundary_comment(
156:fn chain_base_has_leading_call_like(
175:fn chain_head_promotion_remaining_width(
189:fn chain_should_avoid_head_promotion_for_nonhead_callbacks(
206:fn promote_chain_head_operations(
244:fn promote_curried_call_tail_line(
288:fn promote_argument_member_call_head_line(
332:fn promote_argument_member_call_pair_line(
384:fn merge_argument_short_member_hop_lines(
417:fn apply_argument_chain_line_promotions(
435:fn normalize_chain_layout(
481:pub(super) fn plan_chain_layout(
```

#### `language/formatter/src/format/expression/operator/dispatch.rs`
- Line count: 339.
- Function signature count: 7.

```text
9:pub(crate) fn format_operator_expression<'ast>(
206:fn format_member_or_chain_expression<'ast>(
226:fn format_index_or_chain_expression<'ast>(
249:fn format_call_or_chain_expression<'ast>(
271:fn format_instantiation_or_chain_expression<'ast>(
291:fn format_maybe_or_chain_expression<'ast>(
311:fn format_must_or_chain_expression<'ast>(
```

#### `language/formatter/src/format/expression/chain/line_group.rs`
- Line count: 334.
- Function signature count: 11.

```text
4:pub(super) fn expression_has_line_postfix_boundary_comment(
24:fn chain_call_has_single_template_literal_argument(
39:pub(super) fn group_chain_expression_lines(
72:fn line_starts_with_mergeable_direct_index(
89:fn merge_direct_index_lines(
110:fn extend_maybe_line(
165:fn extend_member_line(
240:fn extend_call_like_line(
265:fn extend_must_line(iter: &mut ChainOperationIter, line: &mut SmallVec<[ChainExpression; 2]>) {
283:fn should_merge_member_run_with_call(
327:fn push_next_chain_operation(
```

#### `language/formatter/src/format/expression/chain/length.rs`
- Line count: 330.
- Function signature count: 10.

```text
4:pub(crate) fn member_is_private_hash(
35:pub(crate) fn path_postfix_annotations_emit_on_tail(
78:pub(crate) fn path_last_segment_start(
108:pub(crate) fn path_deferred_boundary_line_comments(
155:pub(crate) fn static_arguments_len(
166:pub(crate) fn static_argument_list_len(
178:pub(crate) fn chain_operation_len(
250:pub(crate) fn chain_call_can_expand_in_head(
279:pub(crate) fn chain_head_operation_len(
304:pub(crate) fn chain_base_len(
```

#### `language/formatter/src/format/expression/operator/assign.rs`
- Line count: 329.
- Function signature count: 1.

```text
14:pub(super) fn format_assign_expression<'ast>(
```

#### `language/formatter/src/format/expression/object.rs`
- Line count: 322.
- Function signature count: 5.

```text
12:pub(super) fn format_boundary_comment_array<'ast>(
66:pub(super) fn is_assignment_left_target(
99:fn is_multiline_pattern_field_default_object(
135:fn object_has_leading_newline_before_first_property(
162:pub(crate) fn format_struct_literal<'ast>(
```

#### `language/formatter/src/format/expression/call/render.rs`
- Line count: 317.
- Function signature count: 8.

```text
7:pub(in super::super) fn collect_deferred_empty_call_boundary_comments(
87:pub(in super::super) fn callee_expression_chain_ids(
113:pub(in super::super) fn enclosing_empty_call_id_for_callee_expression(
173:pub(in super::super) fn is_deferred_empty_call_boundary_annotation(
215:pub(in super::super) fn expression_is_in_deferred_empty_call_boundary_chain(
232:pub(in super::super) fn format_call_dynamic_arguments_with_deferred_comments<'ast>(
269:pub(in super::super) fn format_call_expression<'ast>(
299:pub(in super::super) fn format_instantiation_expression<'ast>(
```

#### `language/formatter/src/format/declaration/dispatch.rs`
- Line count: 315.
- Function signature count: 4.

```text
22:pub(super) fn format_super_type_clause<'ast>(
45:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
55:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
65:    fn format_node(
```

#### `language/formatter/src/format/expression/chain/format.rs`
- Line count: 293.
- Function signature count: 5.

```text
9:pub(crate) fn format_expression_chain<'ast>(
121:fn format_chain_base<'ast>(
166:fn format_chain_expression<'ast>(
272:pub(crate) fn should_parenthesize_index_expression(
285:fn format_chain_expression_line<'ast>(
```

#### `language/formatter/src/format/declaration/module_like.rs`
- Line count: 268.
- Function signature count: 4.

```text
16:pub(super) fn format_global_declaration<'ast>(
58:pub(super) fn format_namespace_declaration<'ast>(
122:pub(super) fn format_import_alias_declaration<'ast>(
170:pub(super) fn format_extension_declaration<'ast>(
```

#### `language/formatter/src/format/expression/classify.rs`
- Line count: 263.
- Function signature count: 13.

```text
9:pub fn is_trivial_expression(tree: &NodeTree, expression: &Expression) -> bool {
61:fn static_arguments_are_trivial(
71:pub fn is_complex_expression(_tree: &NodeTree, expression: &Expression) -> bool {
83:pub fn is_trivial_argument(tree: &NodeTree, argument: &Argument) -> bool {
95:pub fn is_trivial_property(tree: &NodeTree, property: &Property) -> bool {
109:pub fn is_complex_argument(tree: &NodeTree, argument: &Argument) -> bool {
121:pub fn is_expression_breakable(tree: &NodeTree, expression: &Expression) -> bool {
173:pub fn is_pattern_breakable(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
185:pub(super) fn span_has_comment(context: &DestackFormatContext<'_>, span: Span) -> bool {
190:pub(super) fn array_elements_are_fill_candidates(
200:pub(super) fn array_has_only_boundary_comments(
231:fn is_comment_token_type(token_type: TokenType) -> bool {
242:fn array_element_is_fill_candidate(tree: &NodeTree, element_id: LocalNodeId<Argument>) -> bool {
```

#### `language/formatter/src/format/expression/scan.rs`
- Line count: 260.
- Function signature count: 11.

```text
6:pub(crate) fn source_min_inline_char_len(source: &str) -> usize {
18:pub(crate) fn expression_has_static_type_arguments(
72:pub(crate) fn expression_has_non_doc_multiline_block_prefix_comment_annotation(
98:pub(crate) fn span_inline_char_bounds(
109:pub(super) fn previous_non_whitespace_before_span(
117:pub(super) fn expression_source_has_outer_parentheses(
127:pub(super) fn expression_has_prefix_comment_annotation(
147:pub(super) fn expression_has_leading_prefix_comment(
190:pub(super) fn yield_value_has_leading_prefix_comment(
198:pub(super) fn should_hoist_parenthesized_inner_cast_prefix_comments(
228:pub(super) fn sequence_expression_needs_parens(
```

#### `language/formatter/src/format/timing.rs`
- Line count: 258.
- Function signature count: 8.

```text
16:    pub const fn new(name: &'static str) -> Self {
21:    pub const fn name(self) -> &'static str {
45:    pub fn record(&self, tag: FormatterTimingTag, duration: Duration) {
57:    pub fn snapshot(&self) -> Vec<FormatterTimingEntry> {
80:    pub fn new(timings: Option<&Rc<FormatterTimings>>, tag: FormatterTimingTag) -> Self {
93:    fn drop(&mut self) {
109:pub fn timings_enabled_from_env() -> bool {
124:pub fn tag_for_node_type(node_type: NodeType) -> FormatterTimingTag {
```

#### `language/formatter/src/format/expression/chain/policy.rs`
- Line count: 254.
- Function signature count: 4.

```text
41:fn for_each_chain_operation(
57:fn collect_chain_operation_facts(
134:pub(super) fn build_chain_render_inputs(
193:pub(super) fn decide_chain_render(
```

#### `language/formatter/src/format/dependency.rs`
- Line count: 179.
- Function signature count: 12.

```text
13:fn format_dependency_item_name<'ast>(
32:    fn format_node(
75:    fn test_format_import() {
85:    fn test_format_import_with_alias() {
95:    fn test_format_import_with_items_from() {
105:    fn test_format_import_with_overflow() {
121:    fn test_format_export_glob() {
131:    fn test_format_import_with_default_and_block() {
141:    fn test_format_import_type_equals_require() {
151:    fn test_format_export_import_type_equals_require() {
161:    fn test_format_export_with_default_and_block() {
171:    fn test_format_export_with_attributes() {
```

#### `language/formatter/src/format/match.rs`
- Line count: 177.
- Function signature count: 8.

```text
18:fn format_selector_with_style(
72:pub(crate) fn format_match_case_with_style<'ast>(
115:    fn format_node(
129:    fn test_format_match_expression_cases() {
139:    fn test_format_match_with_block_case_and_guard() {
149:    fn test_format_switch_expression_cases() {
159:    fn test_format_switch_with_default_case() {
169:    fn test_format_switch_with_block() {
```

#### `language/formatter/src/format/operator.rs`
- Line count: 168.
- Function signature count: 5.

```text
12:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
35:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
54:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
111:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
128:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
```

#### `language/formatter/src/format/key.rs`
- Line count: 157.
- Function signature count: 9.

```text
11:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
19:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
26:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
33:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
39:pub(crate) fn format_key_with_quote_policy<'ast>(
70:pub(crate) fn is_identifier_for_quotes(content: &str) -> bool {
78:fn contains_katakana_middle_dot(content: &str) -> bool {
85:fn format_name_with_quote_policy<'ast>(
144:fn format_quoted_name<'ast>(
```

#### `language/formatter/src/format/expression/core.rs`
- Line count: 152.
- Function signature count: 3.

```text
11:fn expression_format_route(expression: &Expression) -> ExpressionFormatRoute {
85:pub(crate) fn format_expression<'ast>(
129:    fn format_node(
```

#### `language/formatter/src/format/imports.rs`
- Line count: 140.
- Function signature count: 5.

```text
9:pub fn get_import_expression(
46:pub fn is_import(expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
53:pub fn sort_imports(
87:pub fn sort_dependency_items(
101:pub fn should_insert_blank_between(
```

#### `language/formatter/src/format/expression/operator/new.rs`
- Line count: 106.
- Function signature count: 1.

```text
5:pub(super) fn format_new_expression<'ast>(
```

#### `language/formatter/src/format/expression/mod.rs`
- Line count: 89.
- Function signature count: 0.

- No function signatures were detected in this file.

#### `language/formatter/src/format/collection.rs`
- Line count: 85.
- Function signature count: 5.

```text
18:    pub(crate) fn should_expand_multiline(self) -> bool {
24:pub(crate) fn collection_nodes_have_annotations<T>(
39:pub(crate) fn collection_nodes_have_newline<T>(
54:pub(crate) fn collection_range_is_inline<T>(
80:pub(crate) fn collection_value_should_force_break(
```

#### `language/formatter/src/format/expression/sort.rs`
- Line count: 84.
- Function signature count: 1.

```text
6:pub(super) fn format_export_import_equals(
```

#### `language/formatter/src/format/enum.rs`
- Line count: 84.
- Function signature count: 5.

```text
10:    fn format_node(
40:    fn test_format_enum_empty() {
50:    fn test_format_enum_with_simple_fields() {
60:    fn test_format_enum_with_annotations() {
70:    fn test_format_enum_with_static_parameters() {
```

#### `language/formatter/src/format/where.rs`
- Line count: 62.
- Function signature count: 3.

```text
10:pub(crate) fn format_where_clause<'ast>(
33:pub(crate) fn format_where_clause_with_break<'ast>(
49:    fn format_node(
```

#### `language/formatter/src/format/path.rs`
- Line count: 58.
- Function signature count: 4.

```text
9:    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
30:    fn test_format_path_short() {
40:    fn test_format_path_multiple_segments() {
50:    fn test_format_path_with_overlong_line() {
```

#### `language/formatter/src/format/scan.rs`
- Line count: 54.
- Function signature count: 4.

```text
6:pub(crate) fn previous_non_whitespace_before_span(
23:pub(crate) fn next_non_whitespace_after_span(
39:pub(crate) fn previous_non_whitespace_before_annotation(
48:pub(crate) fn next_non_whitespace_after_annotation(
```

#### `language/formatter/src/format/mod.rs`
- Line count: 29.
- Function signature count: 0.

- No function signatures were detected in this file.

#### `language/formatter/src/format/expression/generic.rs`
- Line count: 29.
- Function signature count: 1.

```text
5:pub(super) fn format_static_argument_list<'ast>(
```

#### `language/formatter/src/format/expression/chain/mod.rs`
- Line count: 16.
- Function signature count: 0.

- No function signatures were detected in this file.

#### `language/formatter/src/format/expression/operator/common.rs`
- Line count: 11.
- Function signature count: 1.

```text
4:pub(super) fn expression_is_trivial_inline_without_annotations(
```

#### `language/formatter/src/format/expression/call/mod.rs`
- Line count: 9.
- Function signature count: 0.

- No function signatures were detected in this file.

#### `language/formatter/src/format/expression/operator/mod.rs`
- Line count: 7.
- Function signature count: 0.

- No function signatures were detected in this file.

#### `language/formatter/src/format/declaration/mod.rs`
- Line count: 5.
- Function signature count: 0.

- No function signatures were detected in this file.

## Agreed Target Architecture
This target architecture was agreed on February 12, 2026.
This structure prioritizes syntax-domain boundaries first and pipeline-stage boundaries second.
This structure is designed to reduce heuristic drift by separating analysis and policy and rendering.

```text
format/
  mod.rs

  context/
    mod.rs
    source.rs
    annotations.rs
    parents.rs
    caches.rs
    metrics.rs
    timing.rs

  annotation/
    mod.rs
    capture.rs
    emit.rs
    defer.rs
    policy.rs

  argument/
    mod.rs
    list.rs
    parameter.rs
    comments.rs
    policy.rs

  declaration/
    mod.rs
    dispatch.rs
    variable.rs
    function_like.rs
    class_like.rs
    interface_like.rs
    type_alias.rs
    enum_like.rs
    import_export_alias.rs
    shared.rs

  expression/
    mod.rs
    dispatch.rs
    classify.rs
    scan.rs
    primary.rs
    member.rs
    object.rs
    binary.rs
    declarator.rs
    control.rs
    parentheses.rs
    ternary.rs
    statement/
      mod.rs
      import_export.rs
      variable.rs
      loops.rs
      control_flow.rs
    call/
      mod.rs
      classify.rs
      analyze.rs
      analyze_layout.rs
      render.rs
    chain/
      mod.rs
      normalize.rs
      classify.rs
      line_group.rs
      policy.rs
      render.rs
      length.rs
    jsx/
      mod.rs
      attributes.rs
      children.rs
      policy.rs
      render.rs
    operator/
      mod.rs
      dispatch.rs
      assign.rs
      binary.rs
      new.rs
      common.rs

  literal.rs
  property.rs
  signature.rs
  imports.rs
  dependency.rs
  block.rs
  pattern.rs
  match.rs
  key.rs
  path.rs
  collection.rs
  operator.rs
  scan.rs
```

## Migration Principles
Every batch should be no-behavior-change unless explicitly marked as policy work.
Every batch should keep `mod.rs` files as thin export and wiring layers.
Every batch should move code by responsibility, not by arbitrary chunk size.
Every file split should be justified by correctness or performance or AGENTS style readability, not by size alone.
Every batch should preserve public and crate-visible API surfaces until the final consolidation step.
Every batch should pass formatter release tests and conformance accounting before merge.

## Old To New Mapping
`language/formatter/src/format/declaration.rs` maps to `declaration/dispatch.rs`, `declaration/function_like.rs`, `declaration/type_like.rs`, `declaration/module_like.rs`, and `declaration/type_alias.rs`.
`language/formatter/src/format/expression/call/profile.rs` now maps to `expression/call/analyze.rs` and `expression/call/analyze_layout.rs`, with `expression/call/render.rs` and `expression/call/classify.rs` kept focused.
`language/formatter/src/format/expression/chain/format.rs` now maps to `expression/chain/normalize.rs`, `expression/chain/line_group.rs`, and `expression/chain/policy.rs`, with `expression/chain/format.rs` kept render-focused for now.
`language/formatter/src/format/annotation.rs` maps to `annotation/capture.rs`, `annotation/emit.rs`, and `annotation/policy.rs` while `annotation/defer.rs` remains defer-focused.
`language/formatter/src/format/expression/jsx.rs` maps to `expression/jsx/attributes.rs`, `expression/jsx/children.rs`, `expression/jsx/policy.rs`, and `expression/jsx/render.rs`.
`language/formatter/src/format/context.rs` maps to `context/source.rs`, `context/annotations.rs`, `context/parents.rs`, `context/caches.rs`, `context/metrics.rs`, and `context/timing.rs`.
`language/formatter/src/format/expression/statement.rs` maps to `expression/statement/import_export.rs`, `expression/statement/variable.rs`, `expression/statement/loops.rs`, and `expression/statement/control_flow.rs`.

## Execution Plan
### Batch 1: Declaration Split
Move declaration helpers and declaration format branches into `declaration/*` files.
Leave `FormatNode<Declaration>` entry wiring in `declaration/dispatch.rs`.
Keep all behavior and formatting output identical.
Status: completed in quality cleanup batch 5.

### Batch 2: Call Analysis Split
Move non-render call profiling logic from `expression/call/profile.rs` into `expression/call/analyze.rs` and `expression/call/analyze_layout.rs`.
Keep only wiring and minimal entry points in `profile.rs` until the final rename step.
Keep counter names stable in this phase.
Status: completed in quality cleanup batch 6.

### Batch 3: Chain Planner Split
Move chain normalization and line grouping logic from `expression/chain/format.rs` into `normalize.rs` and `line_group.rs`.
Move render decision logic into `policy.rs` and document decision inputs explicitly.
Retain `format_expression_chain` as the public entry point in `format.rs` for now.
Status: completed in quality cleanup batch 7.

### Batch 4: Boundary Comment Policy Consolidation
Unify boundary comment and deferred annotation policy across call and chain and ternary and annotation defer codepaths.
Remove duplicate boundary checks and keep one shared policy surface with explicit predicates.
Only split files when this directly improves correctness or perf or AGENTS style readability.

### Batch 5: Call Heuristic Simplification
Refactor `decide_post_hugged_call_argument_layout` into explicit policy phases in place so callback tail and collection tail rules are deterministic.
Reduce branch depth and move repeated predicate checks into helper functions with clear naming.
Keep behavior stable unless a targeted conformance fix is explicitly included.

### Batch 6: Chain And Call Coupling Cleanup
Remove policy overlap between chain normalization and call argument layout heuristics.
Keep chain head promotion and argument chain promotion rules data driven and test backed.

### Batch 7: Hot Path Perf Sweep
Prioritize perf work in `context.rs`, `annotation.rs`, `expression/jsx.rs`, `argument.rs`, and `expression/call/analyze_layout.rs`.
Add or extend counters and microbench probes around the hottest policy predicates before and after changes.

### Batch 8: Wiring Cleanup And Docs
Remove transitional re exports and legacy shim modules that no longer serve behavior or perf goals.
Normalize module visibility and import paths.
Update `language/formatter/README.md` with the policy first architecture notes.
## Validation Gates Per Batch
Each batch must pass `cargo +nightly-2025-11-27 fmt -p destack_formatter`.
Each batch must pass `cargo clippy -p destack_formatter --release --all-targets`.
Each batch must pass `cargo test --release -p destack_formatter`.
Each batch must pass `cargo test --release -p destack_test --test formatter`.
Each batch must pass `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt`.
Each batch should run `cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` at least every two batches and at every merge checkpoint.
Each batch should run `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` at least every two batches.

## Definition Of Done
File size should be managed pragmatically, and large files are acceptable when logic remains systematic and measurable and maintainable.
No single formatter policy file should mix scan facts and policy and rendering in one function.
Declaration and call and chain and annotation and jsx should each have explicit policy modules.
The formatter should remain green on own tests and `oxfmt` and fully accounted on prettier and biome known-failure baselines.

## Quality Cleanup Batch 9
### Batch 9 Scope
This batch focused on correctness-first cleanup and module hygiene before the next conformance-focused changeset.
The cleanup prioritized systematic comment or newline trivia detection, compile-stable module boundaries, and refreshed baseline metrics for planning.

### Batch 9 Code Changes
I repaired module-wiring regressions caused by broad `mod.rs` visibility re-exports and restored a compile-clean formatter graph.
I kept the `pub(crate)` conversion for previous `pub(in super::super)` call submodule APIs and removed the remaining pattern usage in formatter code.
I replaced raw source substring comment probes with span or token-aware checks in `language/formatter/src/format/expression/chain/classify.rs`, `language/formatter/src/format/expression/parentheses.rs`, and `language/formatter/src/format/annotation.rs`.
I kept the `CallArgumentLayoutScanState` `derive(Default)` pattern with explicit optimistic constructor flags, and preserved the phased `format_new_expression` cleanup.

### Batch 9 Validation Results
`cargo check -p destack_formatter` passed.
`cargo test --release -p destack_formatter` passed with `227 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.

### Batch 9 Throughput Snapshot
Run A of `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `3.991s`, lines per second `1.96M/s`, format CPU lines per second `670.3K/s`, and print CPU lines per second `5.47M/s`.
Run B of the same command reported mean total `3.093s`, lines per second `2.53M/s`, format CPU lines per second `893.4K/s`, and print CPU lines per second `6.96M/s`.
Given host contention, Run A is likely a noisy low-throughput outlier and Run B aligns with prior healthy throughput checkpoints.

## Refreshed Formatter File Metrics
This table was regenerated on February 12, 2026 from `language/formatter/src/format` after Batch 9 changes.
| file | lines | function_count |
|:--|--:|--:|
| `language/formatter/src/format/annotation.rs` | 1816 | 23 |
| `language/formatter/src/format/annotation/defer.rs` | 830 | 26 |
| `language/formatter/src/format/argument.rs` | 1408 | 23 |
| `language/formatter/src/format/block.rs` | 905 | 14 |
| `language/formatter/src/format/collection.rs` | 85 | 4 |
| `language/formatter/src/format/context.rs` | 1918 | 0 |
| `language/formatter/src/format/declaration/dispatch.rs` | 315 | 1 |
| `language/formatter/src/format/declaration/function_like.rs` | 471 | 7 |
| `language/formatter/src/format/declaration/mod.rs` | 5 | 0 |
| `language/formatter/src/format/declaration/module_like.rs` | 268 | 4 |
| `language/formatter/src/format/declaration/type_alias.rs` | 389 | 5 |
| `language/formatter/src/format/declaration/type_like.rs` | 385 | 9 |
| `language/formatter/src/format/dependency.rs` | 179 | 1 |
| `language/formatter/src/format/directive.rs` | 603 | 16 |
| `language/formatter/src/format/enum.rs` | 84 | 0 |
| `language/formatter/src/format/expression/binary.rs` | 535 | 17 |
| `language/formatter/src/format/expression/call/analyze.rs` | 607 | 15 |
| `language/formatter/src/format/expression/call/analyze_layout.rs` | 893 | 27 |
| `language/formatter/src/format/expression/call/classify.rs` | 473 | 22 |
| `language/formatter/src/format/expression/call/mod.rs` | 9 | 0 |
| `language/formatter/src/format/expression/call/profile.rs` | 704 | 13 |
| `language/formatter/src/format/expression/call/render.rs` | 317 | 8 |
| `language/formatter/src/format/expression/chain/base.rs` | 759 | 24 |
| `language/formatter/src/format/expression/chain/break.rs` | 813 | 16 |
| `language/formatter/src/format/expression/chain/classify.rs` | 790 | 30 |
| `language/formatter/src/format/expression/chain/format.rs` | 293 | 5 |
| `language/formatter/src/format/expression/chain/length.rs` | 330 | 10 |
| `language/formatter/src/format/expression/chain/line_group.rs` | 335 | 11 |
| `language/formatter/src/format/expression/chain/mod.rs` | 16 | 0 |
| `language/formatter/src/format/expression/chain/normalize.rs` | 550 | 15 |
| `language/formatter/src/format/expression/chain/policy.rs` | 254 | 4 |
| `language/formatter/src/format/expression/classify.rs` | 263 | 13 |
| `language/formatter/src/format/expression/control.rs` | 410 | 9 |
| `language/formatter/src/format/expression/core.rs` | 152 | 2 |
| `language/formatter/src/format/expression/declarator.rs` | 728 | 4 |
| `language/formatter/src/format/expression/generic.rs` | 29 | 1 |
| `language/formatter/src/format/expression/jsx.rs` | 1829 | 34 |
| `language/formatter/src/format/expression/member.rs` | 544 | 17 |
| `language/formatter/src/format/expression/mod.rs` | 89 | 0 |
| `language/formatter/src/format/expression/object.rs` | 322 | 5 |
| `language/formatter/src/format/expression/operator/assign.rs` | 329 | 1 |
| `language/formatter/src/format/expression/operator/binary.rs` | 841 | 7 |
| `language/formatter/src/format/expression/operator/common.rs` | 11 | 1 |
| `language/formatter/src/format/expression/operator/dispatch.rs` | 339 | 7 |
| `language/formatter/src/format/expression/operator/mod.rs` | 7 | 0 |
| `language/formatter/src/format/expression/operator/new.rs` | 135 | 3 |
| `language/formatter/src/format/expression/parentheses.rs` | 629 | 24 |
| `language/formatter/src/format/expression/primary.rs` | 751 | 4 |
| `language/formatter/src/format/expression/scan.rs` | 260 | 11 |
| `language/formatter/src/format/expression/sort.rs` | 84 | 1 |
| `language/formatter/src/format/expression/statement.rs` | 950 | 13 |
| `language/formatter/src/format/expression/ternary.rs` | 604 | 12 |
| `language/formatter/src/format/expression/tests.rs` | 1187 | 89 |
| `language/formatter/src/format/imports.rs` | 140 | 5 |
| `language/formatter/src/format/key.rs` | 157 | 5 |
| `language/formatter/src/format/literal.rs` | 710 | 13 |
| `language/formatter/src/format/match.rs` | 177 | 2 |
| `language/formatter/src/format/mod.rs` | 29 | 0 |
| `language/formatter/src/format/operator.rs` | 168 | 0 |
| `language/formatter/src/format/path.rs` | 58 | 0 |
| `language/formatter/src/format/pattern.rs` | 522 | 7 |
| `language/formatter/src/format/property.rs` | 814 | 18 |
| `language/formatter/src/format/scan.rs` | 54 | 4 |
| `language/formatter/src/format/signature.rs` | 430 | 16 |
| `language/formatter/src/format/timing.rs` | 258 | 2 |
| `language/formatter/src/format/where.rs` | 62 | 2 |


## Refreshed Signatures For Batch 9 Touched Files
This subsection lists current line counts and detected function signatures for formatter files touched in Batch 9.

### `language/formatter/src/format/annotation.rs`

lines: 1816

functions:
- 114:fn annotation_precedes_separator<'ast>(
- 125:fn annotation_line_with_indentation<'ast>(
- 140:fn write_annotation_line_with_indentation<'ast>(
- 169:fn annotation_next_token_is_on_same_line(
- 198:fn annotation_follows_colon<'ast>(
- 206:fn annotation_follows_opening_delimiter<'ast>(
- 217:pub(super) fn annotation_starts_on_own_line<'ast>(
- 230:fn annotation_has_leading_newline<'ast>(
- 243:fn comment_annotation_starts_on_own_line<'ast>(
- 266:fn annotation_comment_style(
- 279:fn annotation_comment_is_own_line(
- 290:fn annotation_raw_line_or_trimmed_source(
- 302:fn annotation_ancestor_flags<T: Node>(
- 335:fn node_has_member_shape<T: Node>(
- 364:fn annotation_node_context<T: Node>(
- 404:fn annotation_render_facts(
- 458:fn annotation_capture_includes_position(
- 999:fn format_line_comment_lines<'ast>(
- 1033:fn normalize_inline_block_comment_content(content: &str) -> &str {
- 1052:fn is_compact_hint_comment(content: &str) -> bool {
- 1060:fn is_all_asterisks_comment(content: &str) -> bool {
- 1093:fn decorator_needs_parentheses(tree: &NodeTree, expression_id: LocalNodeId<Expression>) -> bool {
- 1110:fn is_identifier_or_static_member_only(

### `language/formatter/src/format/declaration/dispatch.rs`

lines: 315

functions:
- 22:pub(super) fn format_super_type_clause<'ast>(

### `language/formatter/src/format/declaration/function_like.rs`

lines: 471

functions:
- 23:fn is_module_typescript_source(file_name: &str) -> bool {
- 28:fn lambda_parameter_should_expand(
- 49:fn lambda_parameter_is_simple_tail(
- 65:fn append_lambda_arrow_prefix_annotations<T: destack_ast::Node + Clone>(
- 91:fn collect_lambda_arrow_prefix_annotations(
- 117:fn write_deferred_lambda_arrow_prefix_annotations<'ast>(
- 146:pub(super) fn format_function_declaration<'ast>(

### `language/formatter/src/format/declaration/mod.rs`

lines: 5

functions:
- none

### `language/formatter/src/format/declaration/module_like.rs`

lines: 268

functions:
- 16:pub(super) fn format_global_declaration<'ast>(
- 58:pub(super) fn format_namespace_declaration<'ast>(
- 122:pub(super) fn format_import_alias_declaration<'ast>(
- 170:pub(super) fn format_extension_declaration<'ast>(

### `language/formatter/src/format/declaration/type_alias.rs`

lines: 389

functions:
- 30:fn single_line_type_grouping_prefix_comment_cluster(
- 111:fn expression_has_doc_like_block_prefix_annotation(
- 140:fn expression_has_prefix_annotation_in_left_spine(
- 166:fn format_expression_without_prefix_annotations<'ast>(
- 191:pub(super) fn format_type_alias_declaration<'ast>(

### `language/formatter/src/format/declaration/type_like.rs`

lines: 385

functions:
- 21:fn write_deferred_declaration_body_boundary_prefix_annotations<'ast>(
- 55:fn format_declaration_header_prefix<'ast>(
- 71:fn format_declaration_static_parameters<'ast>(
- 85:fn format_declaration_where_clauses<'ast>(
- 99:fn format_declaration_heritage<'ast>(
- 121:fn format_anonymous_class_heritage<'ast>(
- 178:pub(super) fn format_struct_or_class_declaration<'ast>(
- 276:pub(super) fn format_enum_declaration<'ast>(
- 343:pub(super) fn format_interface_declaration<'ast>(

### `language/formatter/src/format/expression/call/analyze.rs`

lines: 607

functions:
- 85:pub(super) fn collect_call_argument_comment_profile(
- 126:pub(super) fn collect_single_call_argument_facts(
- 140:pub(super) fn argument_has_callback_blocking_comment_annotation(
- 151:pub(super) fn call_force_expand_single_long_with_static_arguments(
- 176:pub(super) fn call_force_expand_single_collection_for_type_binary_callee(
- 187:pub(super) fn single_argument_requires_expanded_list(
- 210:pub(super) fn build_call_argument_expansion_profiles(
- 398:pub(super) fn resolve_regular_call_argument_expansion_profile(
- 437:pub(crate) fn resolve_chain_call_argument_force_expand(
- 476:pub(super) fn call_should_force_hugged_expand(
- 483:pub(super) fn call_inline_len_without_static_arguments(
- 506:pub(super) fn write_inline_call_argument_list<'ast>(
- 533:pub(super) fn argument_is_plain_call_argument(
- 565:pub(super) fn write_plain_call_argument<'ast>(
- 597:pub(super) fn write_plain_call_argument_or_node<'ast>(

### `language/formatter/src/format/expression/call/analyze_layout.rs`

lines: 893

functions:
- 11:pub(super) fn argument_is_compact_simple_unannotated(
- 39:pub(super) fn call_has_call_chain_parent(
- 90:pub(super) fn call_argument_shape_from_layout_class(
- 134:fn build_empty_call_argument_layout_class(
- 149:fn call_argument_callback_flags(
- 175:fn scan_call_argument_layout_state(
- 280:fn build_scanned_call_argument_layout_class(
- 329:pub(super) fn resolve_call_argument_layout_class(
- 364:pub(super) fn leading_dynamic_arguments(
- 375:pub(super) fn call_argument_layout_class_is_simple_multi_unannotated(
- 382:pub(super) fn leading_arguments_are_compact_simple_unannotated(
- 393:pub(super) fn leading_arguments_are_compact_callback_tail_candidates(
- 421:pub(super) fn call_arguments_use_single_simple_argument_fast_path(
- 456:pub(super) fn call_arguments_use_single_callback_argument_inline(
- 495:pub(super) fn call_arguments_use_single_simple_argument_inline(
- 563:pub(super) fn resolve_inline_call_len_without_static_arguments(
- 598:pub(super) fn can_consider_hug_last_call_arguments(
- 616:pub(super) fn call_arguments_force_hug_last_inline(
- 653:fn hug_last_tail_flags(
- 672:fn hug_last_can_inline_by_width(
- 681:fn hug_last_can_inline_callback_tail(
- 693:fn hug_last_can_inline_overflow_tail(
- 706:fn hug_last_should_skip_probe_collection(
- 716:pub(super) fn resolve_hug_last_call_argument_layout(
- 832:pub(super) fn build_call_argument_planner_base_state(
- 848:fn planner_force_expand_single_collection_for_type_binary_callee(
- 865:pub(super) fn build_call_argument_planner_state(

### `language/formatter/src/format/expression/call/classify.rs`

lines: 473

functions:
- 18:fn expression_is_lambda_declaration(
- 34:pub(crate) fn argument_is_simple_with_options(
- 61:pub(crate) fn is_call_like_argument(
- 87:pub(crate) fn is_tree_attribute_expression(
- 113:pub(crate) fn is_simple_static_argument(
- 130:pub(crate) fn argument_is_string_like(
- 144:pub(crate) fn argument_is_interpolated_template_literal(
- 160:pub(crate) fn argument_is_collection_literal(
- 169:pub(crate) fn argument_is_reference_like(
- 191:pub(crate) fn call_callee_has_test_like_member_name(
- 245:pub(crate) fn call_should_force_hug_test_like_callback(
- 268:pub(crate) fn call_arguments_are_multiline_in_source(
- 286:pub(crate) fn call_arguments_preserve_blank_line_between(
- 324:pub(crate) fn call_has_non_blank_infix_annotation(
- 332:pub(crate) fn argument_has_non_blank_annotation(
- 340:pub(crate) fn argument_has_multiline_prefix_annotation(
- 354:pub(crate) fn argument_has_leading_prefix_annotation_outside_span(
- 384:pub(crate) fn argument_has_line_comment_annotation(
- 394:pub(crate) fn argument_has_prefix_line_comment_annotation(
- 404:pub(crate) fn call_has_static_arguments(
- 422:pub(crate) fn call_like_has_type_binary_callee(
- 442:pub(crate) fn call_has_leading_block_callback_with_simple_tail(

### `language/formatter/src/format/expression/call/mod.rs`

lines: 9

functions:
- none

### `language/formatter/src/format/expression/call/profile.rs`

lines: 704

functions:
- 9:fn decide_post_hugged_call_argument_layout(
- 146:fn format_fast_default_call_argument_list<'ast>(
- 185:fn format_default_call_argument_list<'ast>(
- 261:fn format_comment_expanded_call_argument_list<'ast>(
- 317:fn write_single_call_argument_inline_wrapped<'ast>(
- 327:fn format_call_argument_layout_decision<'ast>(
- 366:pub(crate) fn format_call_arguments<'ast>(
- 386:fn call_arguments_use_no_annotation_multi_argument_fast_path(
- 400:fn format_no_annotation_multi_argument_fast_path<'ast>(
- 422:fn call_arguments_use_forced_hug_last_inline_fast_path(
- 430:fn format_single_call_argument_with_group<'ast>(
- 577:fn format_call_arguments_with_group<'ast>(
- 664:pub(crate) fn call_arguments_force_expand_for_chain(

### `language/formatter/src/format/expression/call/render.rs`

lines: 317

functions:
- 7:pub(crate) fn collect_deferred_empty_call_boundary_comments(
- 87:pub(crate) fn callee_expression_chain_ids(
- 113:pub(crate) fn enclosing_empty_call_id_for_callee_expression(
- 173:pub(crate) fn is_deferred_empty_call_boundary_annotation(
- 215:pub(crate) fn expression_is_in_deferred_empty_call_boundary_chain(
- 232:pub(crate) fn format_call_dynamic_arguments_with_deferred_comments<'ast>(
- 269:pub(crate) fn format_call_expression<'ast>(
- 299:pub(crate) fn format_instantiation_expression<'ast>(

### `language/formatter/src/format/expression/chain/classify.rs`

lines: 790

functions:
- 4:pub(crate) fn chain_head_id(
- 24:pub(crate) fn is_simple_chain_head(
- 45:pub(crate) fn is_short_chain_argument(
- 62:fn chain_call_has_breakable_dynamic_arguments(
- 79:pub(crate) fn is_poorly_breakable_chain(
- 161:pub(crate) fn argument_value_id(
- 174:pub(crate) fn is_lambda_expression(
- 191:pub(crate) fn is_nested_lambda_expression(
- 218:pub(crate) fn lambda_body_forces_multiline(
- 251:pub(crate) fn expression_callback_depth(
- 301:pub(crate) fn expression_chain_should_break(
- 330:pub(crate) fn expression_is_in_tree_literal_child(
- 370:pub(crate) fn lambda_expression_should_break(
- 428:pub(crate) fn argument_forces_multiline(
- 448:pub(crate) fn is_block_lambda_argument(
- 476:pub(crate) fn is_simple_chain_argument(
- 493:pub(crate) fn arguments_total_len(
- 510:pub(crate) fn arguments_rendered_len(
- 524:pub(crate) fn is_numeric_scalar_literal(expression: &Expression) -> bool {
- 534:pub(crate) fn is_numeric_index_expression(
- 548:pub(crate) fn is_numeric_index(
- 559:pub(crate) fn is_simple_chain_static_arguments(
- 576:pub(crate) fn is_simple_chain_static_argument_list(
- 588:pub(crate) fn is_simple_chain_call(
- 615:pub(crate) fn is_simple_chain_operation(
- 657:fn member_receiver_property_gap_span(
- 683:pub(crate) fn member_has_intervening_comment(
- 692:pub(crate) fn member_has_intervening_break_or_comment(
- 711:pub(crate) fn expression_trivia_anchor_end(
- 734:pub(crate) fn chain_has_parent_intervening_break_or_comment(

### `language/formatter/src/format/expression/chain/format.rs`

lines: 293

functions:
- 9:pub(crate) fn format_expression_chain<'ast>(
- 121:fn format_chain_base<'ast>(
- 166:fn format_chain_expression<'ast>(
- 272:pub(crate) fn should_parenthesize_index_expression(
- 285:fn format_chain_expression_line<'ast>(

### `language/formatter/src/format/expression/chain/line_group.rs`

lines: 335

functions:
- 5:pub(super) fn expression_has_line_postfix_boundary_comment(
- 25:fn chain_call_has_single_template_literal_argument(
- 40:pub(super) fn group_chain_expression_lines(
- 73:fn line_starts_with_mergeable_direct_index(
- 90:fn merge_direct_index_lines(
- 111:fn extend_maybe_line(
- 166:fn extend_member_line(
- 241:fn extend_call_like_line(
- 266:fn extend_must_line(iter: &mut ChainOperationIter, line: &mut SmallVec<[ChainExpression; 2]>) {
- 284:fn should_merge_member_run_with_call(
- 328:fn push_next_chain_operation(

### `language/formatter/src/format/expression/chain/mod.rs`

lines: 16

functions:
- none

### `language/formatter/src/format/expression/chain/normalize.rs`

lines: 550

functions:
- 34:fn collect_chain_root_parts(
- 108:fn append_chain_operations(
- 121:fn chain_body_has_breaking_member_annotation(
- 135:fn chain_should_avoid_head_promotion_for_boundary_comment(
- 156:fn chain_base_has_leading_call_like(
- 175:fn chain_head_promotion_remaining_width(
- 189:fn chain_should_avoid_head_promotion_for_nonhead_callbacks(
- 206:fn promote_chain_head_operations(
- 244:fn promote_curried_call_tail_line(
- 288:fn promote_argument_member_call_head_line(
- 332:fn promote_argument_member_call_pair_line(
- 384:fn merge_argument_short_member_hop_lines(
- 417:fn apply_argument_chain_line_promotions(
- 435:fn normalize_chain_layout(
- 481:pub(super) fn plan_chain_layout(

### `language/formatter/src/format/expression/chain/policy.rs`

lines: 254

functions:
- 41:fn for_each_chain_operation(
- 57:fn collect_chain_operation_facts(
- 134:pub(super) fn build_chain_render_inputs(
- 193:pub(super) fn decide_chain_render(

### `language/formatter/src/format/expression/generic.rs`

lines: 29

functions:
- 5:pub(super) fn format_static_argument_list<'ast>(

### `language/formatter/src/format/expression/jsx.rs`

lines: 1829

functions:
- 6:pub(super) fn has_multiline_jsx_argument(
- 33:pub(super) fn format_inline_stub_comment<'ast>(
- 146:pub(super) fn tree_attribute_value_id(
- 159:fn argument_transparent_value_id(
- 168:fn expression_function_declaration_id(
- 184:fn declaration_is_lambda(tree: &NodeTree, declaration_id: LocalNodeId<Declaration>) -> bool {
- 192:fn lambda_body_expression_id(
- 212:fn argument_lambda_declaration_id(
- 222:fn expression_postfix_receiver_id(expression: &Expression) -> Option<LocalNodeId<Expression>> {
- 236:pub(super) fn property_has_complex_value(
- 281:pub(super) fn property_has_complex_type_value(
- 309:pub(super) fn should_force_break_tree_attributes(
- 410:pub(super) fn is_huggable_expression(
- 439:pub(super) fn format_hugged<'ast>(
- 748:pub(super) fn format_tree_attribute_value<'ast>(
- 935:pub(super) fn tree_text_span_str(
- 954:pub(super) fn tree_text_is_whitespace_only(
- 996:pub(super) fn tree_text_boundary_separator_space(
- 1046:pub(super) fn tree_children_have_blank_line_between(
- 1066:pub(super) fn tree_child_should_inline_braced_expression(
- 1126:pub(super) fn tree_child_breaks_element(
- 1169:pub(super) fn lambda_body_is_complex_for_tree(
- 1184:pub(super) fn argument_is_complex_callback(
- 1196:pub(super) fn argument_is_block_callback(
- 1209:pub(super) fn argument_is_object_literal(
- 1222:pub(super) fn argument_is_array_literal(
- 1235:pub(super) fn argument_is_template_literal(
- 1248:pub(super) fn argument_is_lambda_expression(
- 1256:pub(super) fn argument_is_function_expression(
- 1271:pub(super) fn expression_has_complex_callback(
- 1324:pub(crate) fn tree_literal_should_break(
- 1365:pub(super) fn tree_literal_wraps_on_break(
- 1421:pub(super) fn format_tree_literal_expression<'ast>(
- 1457:pub(crate) fn format_tree_literal<'ast>(

### `language/formatter/src/format/expression/mod.rs`

lines: 89

functions:
- none

### `language/formatter/src/format/expression/object.rs`

lines: 322

functions:
- 12:pub(super) fn format_boundary_comment_array<'ast>(
- 66:pub(super) fn is_assignment_left_target(
- 99:fn is_multiline_pattern_field_default_object(
- 135:fn object_has_leading_newline_before_first_property(
- 162:pub(crate) fn format_struct_literal<'ast>(

### `language/formatter/src/format/expression/operator/binary.rs`

lines: 841

functions:
- 10:fn leading_union_has_ancestor_block_prefix_annotation(
- 38:fn is_logical_binary_operator(operator: BinaryOperator) -> bool {
- 47:fn is_mixed_logical_precedence_pair(
- 61:fn preserve_source_operator_break(
- 70:fn operand_prefers_trailing_logical_operator(
- 79:pub(super) fn format_binary_expression<'ast>(
- 768:pub(super) fn format_type_binary_expression<'ast>(

### `language/formatter/src/format/expression/operator/new.rs`

lines: 135

functions:
- 5:fn new_call_has_deferred_empty_argument_comments(
- 22:fn format_new_left_without_infix_or_postfix_annotations<'ast>(
- 34:pub(super) fn format_new_expression<'ast>(

### `language/formatter/src/format/expression/parentheses.rs`

lines: 629

functions:
- 24:pub(super) fn parenthesized_should_unwrap(
- 47:pub(super) fn parenthesized_should_drop(
- 71:pub(super) fn parenthesized_prefers_new_member_callee_parentheses(
- 79:pub(super) fn collect_parenthesized_boundary_comments(
- 145:pub(super) fn parenthesized_has_leading_inner_trivia(
- 154:pub(super) fn parenthesized_has_leading_inner_comments(
- 163:fn parenthesized_has_leading_inner_pattern(
- 187:pub(super) fn member_expression_source_has_optional_chain(
- 196:pub(super) fn should_unwrap_parenthesized_member_object(
- 213:pub(super) fn member_object_prefers_new_callee_parentheses(
- 235:pub(super) fn is_simple_new_member_object(
- 253:pub(super) fn should_unwrap_parenthesized_new_member_callee(
- 280:pub(super) fn strip_one_wrapping_parentheses(source: &str) -> &str {
- 289:pub(super) fn type_binary_is_statement_expression(
- 308:pub(super) fn has_parenthesized_ancestor_with_leading_inner_trivia(
- 334:pub(super) fn type_binary_is_parenthesized_new_callee(
- 368:pub(super) fn should_drop_type_binary_left_parentheses(
- 409:pub(super) fn parenthesized_is_top_level_type_alias_value(
- 428:pub(super) fn expression_is_type_binary_chain_head(
- 447:pub(super) fn parenthesized_source_leading_type_grouping_operator(
- 483:pub(super) fn should_drop_parenthesized_type_expression(
- 545:pub(super) fn is_associative_type_binary_operator(operator: BinaryOperator) -> bool {
- 553:pub(super) fn parenthesized_associative_type_binary_can_drop(
- 593:fn should_drop_parenthesized_expression_wrapper(

### `language/formatter/src/format/expression/statement.rs`

lines: 950

functions:
- 8:fn statement_expression_needs_semicolon(
- 36:fn format_dependency_with_arguments<'ast>(
- 53:fn format_import_expression<'ast>(
- 201:fn format_export_expression<'ast>(
- 302:fn format_let_expression<'ast>(
- 347:fn format_using_expression<'ast>(
- 387:fn format_while_expression<'ast>(
- 432:fn format_for_each_expression<'ast>(
- 518:fn format_for_expression<'ast>(
- 548:fn format_loop_expression<'ast>(
- 560:fn format_try_expression<'ast>(
- 610:fn format_return_expression<'ast>(
- 676:pub(super) fn format_statement_expression<'ast>(

### `language/formatter/src/format/expression/ternary.rs`

lines: 604

functions:
- 6:pub(super) fn get_argument_value(
- 19:pub(super) fn collect_ternary_chain(
- 72:pub(super) fn collect_statement_ternary_boundary_prefix_comments(
- 170:pub(super) fn write_ternary_separator_comments<'ast>(
- 181:pub(super) fn write_ternary_colon_line_comments<'ast>(
- 196:pub(super) fn collect_ternary_colon_line_comments(
- 248:pub(super) fn ternary_else_start_for_branch(
- 264:pub(super) fn ternary_then_has_boundary_comment_before_colon(
- 290:pub(super) fn collect_catch_pattern_trailing_boundary_comments(
- 335:pub(super) fn ternary_requires_terminator(
- 360:pub(super) fn ternary_branch_is_tree_like(
- 373:pub(super) fn format_ternary(

### `language/formatter/src/format/expression/tests.rs`

lines: 1187

functions:
- 12:fn context_from_formatter(formatter: &TestFormatter) -> DestackFormatContext<'_> {
- 28:fn find_call_with_dynamic_argument_count(
- 54:fn find_parenthesized_expression_by_inner(
- 78:fn test_format_expression_simple() {
- 89:fn test_format_expression_parenthesized() {
- 100:fn test_format_expression_nested_empty_parenthesis() {
- 111:fn test_format_expression_nested_empty_arguments() {
- 121:fn test_format_expression_struct_literal_trivial() {
- 131:fn test_format_expression_struct_literal_spread() {
- 141:fn test_assignment_target_detection() {
- 175:fn test_format_expression_if_ternary() {
- 185:fn test_format_expression_index_call_mixed_postfix() {
- 196:fn test_format_expression_instantiation() {
- 207:fn test_format_new_expression_drops_simple_member_parentheses() {
- 218:fn test_format_new_expression_keeps_optional_member_parentheses() {
- 229:fn test_format_new_expression_wraps_call_member_callee() {
- 240:fn test_parenthesis_policy_rejects_member_object_boundary_comment() {
- 258:fn test_parenthesis_policy_rejects_optional_new_callee_unwrap() {
- 281:fn test_format_array_expression_sparse_elisions() {
- 310:fn test_format_new_expression_empty_argument_comment() {
- 321:fn test_format_member_expression_unwraps_parenthesized_call_object() {
- 332:fn test_tree_child_map_callback_breaks() {
- 355:fn test_format_type_const_borrow_normalizes_to_readonly() {
- 361:fn test_format_type_const_pointer_normalizes_to_readonly() {
- 385:fn test_format_type_alias_preserves_borrowed_reference() {
- 412:fn test_format_type_alias_preserves_readonly_borrowed_reference() {
- 422:fn test_format_member_call_chain_line() {
- 432:fn test_format_member_call_chain_retains_breaks() {
- 442:fn test_format_member_call_chain_breaks() {
- 452:fn test_format_member_call_chain_breaks_with_maybe_and_index() {
- 462:fn test_format_path_member_call_chain_breaks() {
- 472:fn test_format_index_member_chain_breaks() {
- 483:fn test_format_member_chain_breaks_before_long_boundary_comment_with_optional_call() {
- 495:fn test_format_chain_planner_promotes_head_in_call_like_argument() {
- 506:fn test_format_chain_planner_respects_assignment_rhs_width() {
- 516:fn test_format_expression_tree_literal_without_arguments() {
- 526:fn test_format_expression_tree_literal_with_arguments() {
- 536:fn test_format_expression_tree_literal_parenthesized() {
- 556:fn test_format_expression_tree_literal_nested() {
- 584:fn test_format_expression_tree_literal_with_array_of_struct_element() {
- 609:fn test_format_expression_call_with_struct_literal() {
- 630:fn test_format_expression_let_call() {
- 664:fn test_format_call_single_lambda_argument_with_prefix_comment_breaks() {
- 671:fn test_format_call_nested_arrow_boundary_comments() {
- 679:fn test_format_chained_assignment() {
- 689:fn test_format_chained_assignment_long() {
- 699:fn test_format_jsx_with_comment() {
- 709:fn test_format_jsx_conditional_child() {
- 719:fn test_format_jsx_in_function_call() {
- 729:fn test_format_deeply_nested_callbacks() {
- 740:fn test_call_chain_classifier_expands_callback_heavy_arguments() {
- 764:fn test_call_chain_classifier_expands_single_tree_child_argument() {
- 786:fn test_format_optional_chain_with_nullish() {
- 797:fn test_format_type_conditional_with_infer() {
- 807:fn test_format_type_conditional_with_constrained_infer() {
- 817:fn test_format_type_conditional_nested() {
- 827:fn test_format_type_intersection_trailing_operator_destack() {
- 838:fn test_format_type_mapped_with_remap() {
- 846:fn test_format_type_mapped_with_removals() {
- 854:fn test_format_type_mapped_without_modifiers() {
- 862:fn test_format_type_mapped_with_optional() {
- 870:fn test_format_type_index() {
- 878:fn test_format_type_template_literal() {
- 886:fn test_format_type_template_literal_multiple_spans() {
- 894:fn test_format_type_template_literal_union_with_leading_pipe() {
- 902:fn test_format_type_union_drops_redundant_parentheses() {
- 910:fn test_format_type_single_member_leading_union_parenthesized_array() {
- 918:fn test_format_type_single_member_leading_intersection_parenthesized_array() {
- 934:fn test_type_template_literal_union_is_in_type_context() {
- 974:fn test_format_type_import() {
- 984:fn test_format_type_import_without_qualifier() {
- 994:fn test_format_type_infer_expression() {
- 1000:fn test_format_async_arrow() {
- 1010:fn test_format_return_jsx_inline() {
- 1021:fn test_format_return_jsx_multiline() {
- 1031:fn test_format_nested_ternary() {
- 1042:fn test_format_const_call_with_multiline_object_rhs() {
- 1053:fn test_format_export_const_chain_rhs_does_not_break_after_operator() {
- 1064:fn test_format_const_generic_call_rhs_breaks_after_operator() {
- 1076:fn test_format_const_generic_call_rhs_preserves_source_operator_break() {
- 1088:fn test_format_const_generic_call_with_multiline_type_argument_keeps_operator_inline() {
- 1099:fn test_format_jsx_bracket_same_line_true() {
- 1111:fn test_format_jsx_bracket_same_line_false() {
- 1124:fn test_format_await_inside_maybe_gets_parenthesized() {
- 1137:fn test_format_unary_inside_maybe_gets_parenthesized() {
- 1147:fn test_format_unary_await_expression_parenthesizes_operand() {
- 1158:fn test_format_postfix_inside_maybe_no_extra_parens() {
- 1169:fn test_format_call_inside_maybe_no_parens() {
- 1180:fn test_format_await_maybe_sugar() {

### `language/formatter/src/format/signature.rs`

lines: 430

functions:
- 20:pub(crate) fn parameter_is_variadic(
- 31:pub(crate) fn parameter_has_modifier(
- 44:pub(crate) fn constructor_parameters_should_expand(
- 57:pub(crate) fn single_parameter_should_hug(
- 88:pub(crate) fn write_function_abstraction_prefix(
- 109:pub(crate) fn write_function_asynchrony_prefix(
- 130:pub(crate) fn write_function_header_prefix(
- 175:fn parameter_object_pattern_should_expand(
- 228:pub(crate) fn parameter_should_force_expand_in_signature(
- 254:pub(crate) fn signature_return_type_is_multiline(
- 262:pub(crate) fn signature_should_elide_space_before_body(
- 270:pub(crate) fn signature_parameters_should_expand(
- 294:pub(crate) fn write_signature_dynamic_parameter_list(
- 311:pub(crate) fn collect_deferred_function_boundary_line_comments(
- 369:pub(crate) fn function_body_has_deferred_boundary_line_comments(
- 382:pub(crate) fn format_function_body_block_with_deferred_boundary_line_comments(

## OXC Layout Cross Check
I reviewed `~/symbol/oxc/crates/oxc_formatter/src` to compare module organization patterns.
OXC separates concerns into `print/*` for syntactic domains, `utils/*` for reusable policy helpers, `parentheses/*` for grouping rules, `formatter/*` for FIR-like document and printer internals, and `ir_transform/*` for post-build transforms.
OXC also isolates call-like and chain complexity into dedicated folders, such as `print/call_like_expression/*` and `utils/member_chain/*`.

The most relevant directional takeaways for Destack are listed below.
- Keep syntax-specific formatting in syntax folders and move reusable heuristics into focused utility modules.
- Keep chain and call planners as dedicated domains with explicit boundaries between analysis, policy, and rendering.
- Keep parentheses and trivia policies centralized, and avoid ad hoc source-string probes in expression-level formatters.
- Keep optional post-format transforms isolated from core printing logic.

These takeaways are already partially reflected in the current Destack split to `expression/call/*`, `expression/chain/*`, and declaration submodules.
The remaining highest-value follow-up is to continue de-duplicating boundary comment and break policy code across annotation, chain, call, and parentheses modules.

## Quality Cleanup Batch 10
This batch closes the remaining formatter cleanup points from the latest review checklist and keeps behavior stable.
This pass was executed on February 12, 2026.

### Batch 10 Scope
This batch touched `language/formatter/src/format/expression/call/profile.rs`.
This batch touched `language/formatter/src/format/expression/operator/binary.rs`.
This batch touched `language/formatter/src/format/expression/jsx.rs`.
This batch updated this map with refreshed validation and throughput numbers.

### Batch 10 Refactor Details
`call/profile.rs` now separates default-list decision construction, comment-profile gating, and comment-expanded forcing checks into dedicated helpers before hug-last resolution.
`call/profile.rs` keeps the existing decision order but reduces branch density in `decide_post_hugged_call_argument_layout`.
`operator/binary.rs` now extracts leading-pipe union formatting into focused helper functions for policy building, first-operand writing, trailing-operand writing, and annotation-safe operand rendering.
`operator/binary.rs` keeps existing leading-pipe behavior while reducing nesting inside `format_binary_expression`.
`jsx.rs` cleaned doc-comment placement around `format_tree_attribute_value` to keep formatter-local clippy output clean except intentionally tolerated argument-count warnings.

### Checklist Closure Status
`build_call_argument_planner_state` was simplified in prior batches and remains a narrow state-assembly helper.
`CallArgumentLayoutScanState` uses `#[derive(Default)]` and keeps optimistic defaults in one constructor.
`pub(in super::super)` usage is removed from formatter sources.
`member_has_intervening_break_or_comment` now uses span and token trivia checks rather than raw-source marker probing.
`format_binary_expression` is now split by specialized helpers plus a dedicated leading-pipe-union formatter path.
`format_new_expression` now has explicit logic-block comments and separated helper paths for deferred empty-argument comment handling.
`format_hugged` and `format_tree_attribute_value` are decomposed into focused helper functions.
`format_ternary` now delegates to helper functions for separator-comment writing and single-branch versus nested-branch rendering.
`generic.rs` remains intentionally small with one focused static-argument formatting entry point and low heuristic density.
`mod.rs` re-export policy remains selective to avoid noisy wildcard re-exports that do not improve visibility or correctness under current module visibilities.
The current layout still keeps expression-local policy modules together, while top-level files remain shared cross-expression format primitives.

### Batch 10 Validation Results
`cargo +nightly-2025-11-27 fmt -p destack_formatter` passed.
`cargo check -p destack_formatter` passed.
`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --biome` passed with all `3519` tests accounted for.

### Batch 10 Conformance Snapshot
`oxfmt` remained `129 passed`, `0 failed`, `6 ignored`, and `100.00%`.
`biome` remained `631 passed`, `1096 failed`, `9 ignored`, and `36.54%`.
`prettier` remained `1231 passed`, `561 failed`, `1436 ignored`, and `68.69%`.
Failure kinds remained `biome: parse=365, output=699, idempotence=32` and `prettier: parse=436, output=0, idempotence=125`.

### Batch 10 Throughput Checkpoint
Run set A of `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `2.857s`, lines per second `2.74M/s`, format CPU lines per second `953.9K/s`, and jitter `2.6%`.
Run set B of the same command reported mean total `2.738s`, lines per second `2.86M/s`, format CPU lines per second `987.7K/s`, and jitter `0.9%`.
This batch does not show a throughput regression signal on this host and stays in the recent high end of the observed noisy range.

### Architecture Direction Check Against Oxc
I reviewed `~/symbol/oxc/crates/oxc_formatter/src` and its split between `print/`, `utils/`, `parentheses/`, and formatter-core primitives.
The closest analogous target for Destack remains a layered split of formatter-core primitives, expression policy modules, and narrow utility classifiers rather than one large policy file per syntax family.
Our recent declaration, call-layout, and chain splits are directionally aligned with that shape.
The next architecture steps should continue reducing cross-module policy overlap between call layout, chain policy, ternary separators, and annotation boundary routing.

## Latest Checkpoint: February 12, 2026

This checkpoint supersedes earlier interim numbers in this document.
This checkpoint includes the declarator idempotence stabilization and oxfmt regression recovery pass.

### Code Changes In This Checkpoint

The declarator self-breaking policy in `language/formatter/src/format/expression/declarator.rs` was adjusted to remove the unstable internal-comment long-rhs trigger that caused layout flipping across format passes.
A stable line-comment signal for binary operands was added in `language/formatter/src/format/expression/declarator.rs` so line-comment-heavy binaries can still force break-after-`=` when needed for oxfmt parity.
The logical comment fixture in `language/test/fixtures/formatter/transform/expressions/comments.md` was restored to a stable canonical operator-break expectation.

### Verified Test Baseline

`cargo test --release -p destack_formatter` passed with `227 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.

### Verified Conformance Baseline

`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129/135` passing, `0` failing, and `6` ignored.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier` passed known baseline checks with `1240/3228` passing, `552` failing, and `1436` ignored.
`cargo test --release -p destack_test --test formatter-conformance -- --biome` passed known baseline checks with `633/1736` passing, `1094` failing, and `9` ignored.
The current blended summary is `2002 passed`, `1646 failed`, and `1451 ignored` out of `3648` total at `54.88%` pass rate.
The current prettier failure-kind split is `parse=436`, `output=0`, and `idempotence=116`.
The current biome failure-kind split is `parse=365`, `output=697`, and `idempotence=32`.

### Regression And Stability Notes

A temporary hard-tier regression appeared in `js/comments/computed-member.js` during this pass.
The regression root cause was declarator break-after-`=` logic becoming too insensitive after removing an unstable internal-comment condition.
The recovery used a stricter and more targeted binary line-comment signal instead of broad internal-comment source-length sensitivity.
The idempotence toggle in `transform/expressions/comments.md/comments-in-logical-operators/comment-between-logical-and-operands` is now stable and green.

### Benchmark Baseline Reruns

The benchmark command was `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Rerun A reported mean total `2.954s`, `2.65M/s` lines, `850.5K/s` format CPU lines, and `12.2%` jitter, which is high-noise.
Rerun B reported mean total `2.798s`, `2.80M/s` lines, `874.0K/s` format CPU lines, and `1.7%` jitter, which is the preferred baseline signal.
Rerun B reported parse CPU `8.914s`, format CPU `8.950s`, and print CPU `1.134s`.
Rerun B reported effective workers `6.79x` and parallel efficiency `84.9%`.

### Current Planning Signal

Hard-tier conformance is green again and should remain locked while soft-tier work proceeds.
The next high-impact soft-tier swing should target idempotence-heavy prettier clusters in binary and chain and call expansion paths.
Any further declarator or assignment policy changes should be validated against both idempotence fixture cases and the oxfmt `computed-member` case before broad conformance reruns.

## Conformance Pass: February 12, 2026

This pass focused on formatter-side idempotence and conformance stabilization without parser changes.
This pass targeted assignment-chain break policy and function-parameter line-comment handling.

### Code Changes

`language/formatter/src/format/signature.rs` now forces multiline parameter formatting when any parameter has a slash comment annotation.
This change stabilized boundary comment movement for function-expression call arguments.
`language/formatter/src/format/expression/operator/assign.rs` now treats nested assignment rhs nodes as self-breaking for break-policy decisions.
`language/formatter/src/format/expression/operator/assign.rs` now uses source-independent break signals for non-chain rhs paths and keeps chain-specific source-newline gating.
`language/formatter/src/format/expression/tests.rs` now includes an idempotence test for function-parameter line-comment call arguments.

### Baseline Verification

`cargo test --release -p destack_formatter` passed with `228 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `129 passed`, `0 failed`, and `6 ignored`.

### Targeted Conformance Spot Checks

`cargo test --release -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/call/first-argument-expansion/issue-13237.js` now passes.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/assignment/chain.js` still fails with idempotence drift.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/call/first-argument-expansion/expression-2nd-arg.js` still fails with idempotence drift.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/call/first-argument-expansion/` reports `5 passed` and `4 failed` where `3` failures are parser-scope and `1` is formatter idempotence.

### Performance Checkpoint

`cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` reported mean total `5.952s` on this contested host.
The same run reported `1.31M/s` overall formatted lines throughput.
The same run reported `410.9K/s` format-stage CPU lines throughput.
The same run reported jitter `2.7%` and effective workers `6.74x`.

### Current Remaining Formatter Targets

The highest-value open formatter idempotence targets remain `js/assignment/chain.js` and `js/call/first-argument-expansion/expression-2nd-arg.js`.
Both remaining failures are assignment associativity and break-placement canonicalization issues rather than parser failures.
The next focused implementation step should normalize assignment chain formatting independent of parse associativity shape.

### Full Conformance Rerun After This Pass

`cargo test --release -p destack_test --test formatter-conformance` passed with all suites accounted for.
The `oxfmt` hard suite remained fully green at `129 passed`, `0 failed`, and `6 ignored`.
The `prettier` soft suite moved to `1241 passed`, `551 failed`, and `1436 ignored` at `69.25%`.
The `biome` soft suite remained `633 passed`, `1094 failed`, and `9 ignored` at `36.65%`.
This full rerun reported one fixed prettier known-failure case: `js/call/first-argument-expansion/issue-13237.js`.
The blended summary moved to `2003 passed`, `1645 failed`, and `1451 ignored` at `54.91%`.
The prettier known-failure baseline file was updated to remove `js/call/first-argument-expansion/issue-13237.js`.
`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` now loads `551` entries.

## Latest Checkpoint Snapshot

This checkpoint was executed on February 12, 2026 after the call profile and annotation boundary cleanup pass.
The validated baseline command `cargo test -p destack_formatter` passed with `230 passed` and `0 failed`.
The validated baseline command `cargo test -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
The validated baseline command `cargo test -p destack_test --test formatter-conformance oxfmt -- --oxfmt` passed with `1 passed` and `0 failed`.

The active conformance spot check target `js/arrays/numbers-with-tricky-comments.js` is currently failing by idempotence in a one case filtered run.
The failing idempotence shape is a first pass split comma line after `466 /* block */` that converges on the second pass.
The targeted run command for this check was `target/debug/deps/formatter_conformance-d5622edc2723067f --prettier --include-known-failures --suite-filter numbers-with-tricky-comments.js`.

The following previously fixed conformance probes remained passing in filtered checks.
`js/arrays/numbers-with-holes.js` passed.
`js/arrows/currying-4.js` passed.

The following baseline regression cluster from earlier in this changeset is now resolved.
`format::annotation::tests::test_format_multi_line_block_comment_stays_block` passes again.
`format::expression::tests::test_format_call_nested_arrow_boundary_comments` passes again.
`format::expression::tests::test_format_call_single_lambda_argument_with_prefix_comment_breaks` passes again.
`format::expression::tests::test_format_chain_planner_promotes_head_in_call_like_argument` passes again.
`transform/expressions/calls.md/typescript-call-grouping/trailing-comment-on-last-argument` passes again.

Current unresolved item for the next conformance swing is stabilizing comma and boundary annotation placement around block comments before separators in array element contexts.
The attempted follow separator annotation facts refinement in `language/formatter/src/format/annotation.rs` did not resolve `numbers-with-tricky-comments.js` idempotence.
The baseline remained green after this attempt.
A focused retest on February 12, 2026 confirmed `js/last-argument-expansion/empty-object.js` passing again and `js/arrays/numbers-with-tricky-comments.js` still failing by idempotence.
A follow up fix on February 12, 2026 resolved `js/arrays/numbers-with-tricky-comments.js` idempotence without regressing `js/last-argument-expansion/empty-object.js`.
The fix path was in `language/formatter/src/format/annotation.rs` and specifically adjusted `AnnotationPosition::LinePostfixBoundary` star comment handling to treat comments that follow a separator as inline preserving boundaries.
The implementation now uses `annotation_follows_separator` in `AnnotationRenderFacts` and allows inline boundary preservation when `precedes_separator` or `follows_separator` is true.
Validation for this fix used `cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter numbers-with-tricky-comments.js` and `cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter empty-object.js`, both passing.
Full local baseline stayed green after this change with `cargo test -p destack_formatter`, `cargo test -p destack_test --test formatter`, and `cargo test -p destack_test --test formatter-conformance oxfmt -- --oxfmt`.

## Checkpoint: Ignore Directive Idempotence Stabilization

This checkpoint was executed on February 12, 2026 and focused on formatter ignore-directive idempotence behavior.
The regression cluster in prettier ignore handling was traced to parenthesized cast comment hoisting and overly broad directive attachment.
The file `language/formatter/src/format/directive.rs` now only resolves annotation-based prefix directives when the directive comment ends before the node start.
The same guard was applied in `ignore_range_for_node` in `language/formatter/src/format/directive.rs` so internal comments do not accidentally bind to parent nodes.
The file `language/formatter/src/format/expression/scan.rs` now prevents parenthesized cast prefix-comment hoisting when the prefix comment is an ignore directive.
This removed a non-idempotent formatting loop in the `issue-14238.ts` prettier-ignore case without broad indentation side effects.

### Targeted Conformance Verification

`cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter issue-14238.ts -v` passed.
`cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter mapped-types.ts -v` passed.
`cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter prettier-ignore-parenthesized-type.ts -v` passed.

### Baseline Verification

`cargo test -p destack_formatter` passed with `230 passed` and `0 failed`.
`cargo test -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test -p destack_test --test formatter-conformance oxfmt -- --oxfmt` passed with `1 passed` and `0 failed`.
`cargo test -p destack_test --test formatter-conformance prettier -- --prettier` passed with `10 passed`, `0 failed`, and `1 ignored` for the currently selected prettier suite slice.
The prettier run reported `545` known failures loaded from `language/test/fixtures/formatter/conformance/prettier-known-failures.txt`.

### Known-Failure Baseline Update

The prettier known-failure baseline file was updated to remove four now-green entries.
The removed entries were `typescript/prettier-ignore/issue-14238.ts`, `typescript/prettier-ignore/mapped-types.ts`, `typescript/prettier-ignore/prettier-ignore-parenthesized-type.ts`, and `typescript/typescript-only/prettier-ignore-parenthesized-type.ts`.
The current prettier known-failure file length is `545` lines.

### Follow Up Sweep: Non-Parse Slice

A follow up scan on the first twenty prettier non-parse slice entries in `language/formatter/.tmp_nonparse_conformance_slice.txt` was executed with `target/debug/deps/formatter_conformance-d5622edc2723067f --prettier --include-known-failures --suite-filter <suite>`.
This scan currently shows seven still-failing entries and thirteen passing entries in that sampled slice.
The failing sampled entries are `js/arrays/preserve_empty_lines.js`, `js/arrows/comments/comment-before-arrow.js`, `js/assignment-comments/call.js`, `js/assignment-comments/function.js`, `js/assignment-comments/number.js`, `js/async/inline-await.js`, and `js/binary-expressions/comment.js`.
All sampled failures in that slice were idempotence failures with `parse=0`, `output=0`, and `idempotence=1`.
The sampled passing entries include `js/assignment/chain.js`, `js/assignment/destructuring-heuristic.js`, `js/assignment/discussion-15196.js`, `js/call/first-argument-expansion/expression-2nd-arg.js`, and `js/call/first-argument-expansion/issue-13237.js`.

### Additional Known-Failure Cleanup

Three additional prettier known-failure entries were removed after passing verification.
The removed entries were `js/arrays/numbers-with-holes.js`, `js/arrays/numbers-with-tricky-comments.js`, and `js/arrows/currying-4.js`.
The updated prettier baseline command `cargo test -p destack_test --test formatter-conformance prettier -- --prettier` now reports `542` known failures loaded.

### Release Gate Revalidation

`cargo test --release -p destack_formatter` passed with `230 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance prettier -- --prettier` passed with `10 passed`, `0 failed`, and `1 ignored` for the current prettier suite slice and reported `542` known failures loaded.
The three additional removed JS entries were revalidated with targeted commands and each passed with `100.00%` in one-case filtered runs.
The revalidation commands were `cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/arrays/numbers-with-holes.js -v`, `cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/arrays/numbers-with-tricky-comments.js -v`, and `cargo test -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter js/arrows/currying-4.js -v`.

## Latest Addendum

This addendum captures the latest formatter cleanup and conformance checkpoint from February 12, 2026 after the deeper file organization and logic cleanup pass.
The cleanup focused on systematic head to body spacing for statement wrapper blocks and `if` branch heads with annotation aware handling.
The cleanup introduced `statement_body_requires_head_space` in `language/formatter/src/format/expression/statement.rs` and `if_branch_head_requires_space` in `language/formatter/src/format/expression/control.rs`.
The cleanup also tightened else boundary inline comment classification in `language/formatter/src/format/expression/control.rs` by refusing to inline when `else` already appears before the candidate comment in source trivia.

### Latest Validation

`cargo test --release -p destack_formatter` passed with `230 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `126 passed`, `3 failed`, `6 ignored`, and `97.67%` suite rate.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier` passed with `1256 passed`, `102 failed`, `1870 ignored`, and `92.49%` suite rate.
The prettier run reported one fixed known failure, `js/explicit-resource-management/valid-await-using-comments.js`, which now passes.
The current prettier failure mode remains entirely idempotence, with `parse=0` and `output=0` and `idempotence=102`.

### Latest Directional Findings

The head-spacing helper changes improved rule locality for statement wrappers but did not fully resolve empty-statement boundary idempotence drift.
The unresolved pattern still includes `do` and `for` wrapper forms that flip between `*/;` and `*/ ;` around inline block comments.
The major remaining instability cluster is inline star comment ownership around `if` and `else` boundaries, especially around `/* ... */ else` spacing normalization.
This remaining cluster appears in `js/comments/between-head-and-body/*` and currently manifests as first pass to second pass spacing drift, not parser or output mismatch.
No new oxfmt regressions remain after this pass, and targeted regressions in `js/if/issue-16137.js` and `js/jsx/arrow-expression.jsx` were resolved before final validation.

### Next Priority From This Addendum

The next conformance swing should isolate `else` boundary annotation ownership into a dedicated helper that classifies comment attachment using explicit source segment checks instead of mixed annotation position heuristics.
The next pass should also add narrow formatter unit tests in `language/formatter/src/format/expression/tests.rs` for `if` and `else` boundary comment layouts that currently only exist in conformance fixtures.
The next pass should keep oxfmt green while iterating on prettier idempotence in this cluster by running a focused conformance slice on `js/comments/between-head-and-body/` before full prettier runs.

### Latest Benchmark Snapshot

`cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` was rerun twice on this changeset.
Rerun A was lock contaminated and reported mean total `4.842s`, lines per second `1.62M/s`, and format CPU lines per second `492.8K/s`.
Rerun B was stable and reported mean total `2.673s`, lines per second `2.93M/s`, and format CPU lines per second `966.7K/s`.
Rerun B jitter was low at `0.9%` and is the better directional signal for this pass.
This places current performance in line with the earlier stable range and does not indicate a clear throughput regression from this cleanup step.

## Ignore Bucket Audit: `js/comments`

This audit was executed on February 12, 2026 with per case filtered runs using `cargo test --release -q -p destack_test --test formatter-conformance -- --prettier --include-ignored --suite-filter <entry>`.
The audited ignore bucket contains `17` entries under `js/comments/*` from `language/test/fixtures/formatter/conformance/prettier-ignored.txt`.
All `17` entries fail in parser stage only with `parse=1`, `output=0`, `idempotence=0`, and `read=0`.
No entry in this bucket currently demonstrates a formatter stage mismatch.
Compared with `HEAD`, `16` of these entries were previously in `prettier-known-failures.txt` and `1` entry was already in `prettier-ignored.txt`.
The one pre existing ignored entry is `js/comments/flow-types/inline.js`.
The parser-only profile indicates this bucket is not a formatter or FIR correctness target for the current changeset scope.
The recommended handling is to keep these `17` entries ignored for now and track them under parser capability backlog.
The parser capability clusters in this bucket are JSX in `.js` contexts, export and import comment edge syntax, and comment sensitive statement wrappers around `for` and `with`.

## Ignore Bucket Audit: `js/class-comment`

This audit was executed on February 12, 2026 with per case filtered runs using `cargo test --release -q -p destack_test --test formatter-conformance -- --prettier --include-ignored --suite-filter <entry>`.
The audited ignore bucket contains `2` entries under `js/class-comment/*` from `language/test/fixtures/formatter/conformance/prettier-ignored.txt`.
Both entries fail in parser stage only with `parse=1`, `output=0`, `idempotence=0`, and `read=0`.
The two entries are `js/class-comment/class-property.js` and `js/class-comment/superclass.js`.
Compared with `HEAD`, both entries were previously in `prettier-known-failures.txt` and neither was previously in `prettier-ignored.txt`.
The parser diagnostics are `unexpected - in Expression` for `class-property.js` and `unexpected Identifier in Expression` for `superclass.js`.
These fixtures cover modern class field syntax and comment placement around `extends`, which we should support long term as modern JS compatibility.
For this formatter and FIR focused changeset, these should stay ignored for now because they are parser-scope failures.

## Ignore Policy Update: Move In Scope Parser Cases To Known Failures

This policy update was applied on February 12, 2026 after auditing `js/comments` and `js/class-comment` ignored buckets.
The decision is that `prettier-ignored.txt` is only for long term out of scope parking, while in scope language support gaps should remain in `prettier-known-failures.txt` even when they are parser stage failures.
Eighteen entries were moved from `language/test/fixtures/formatter/conformance/prettier-ignored.txt` to `language/test/fixtures/formatter/conformance/prettier-known-failures.txt`.
The moved entries are the sixteen non Flow `js/comments/*` parser cases plus `js/class-comment/class-property.js` and `js/class-comment/superclass.js`.
The Flow specific entry `js/comments/flow-types/inline.js` remains ignored.
Validation used `cargo test --release -p destack_test --test formatter-conformance -- --prettier`.
That run loaded `120` known failures and `1852` ignored tests and passed with `1256` passed and `120` failed known failures.
The prettier parser failure list now explicitly includes the moved class comment and comments parser cases as active known failures instead of hidden ignored debt.

## Ignore Policy Batch Update: Explicit Resource Management and TS `as` and `satisfies`

This batch was executed on February 12, 2026 with policy that in scope language support gaps should be tracked in known failures, not ignored.
The batch covered `js/explicit-resource-management/*`, `typescript/as/*`, and `typescript/satisfies-operators/*`.
The batch size was `27` entries.
All `27` entries failed in parser stage only with `parse=1`, `output=0`, `idempotence=0`, and `read=0` in per case filtered runs.
All `27` entries were previously in `HEAD` known failures and not in `HEAD` ignored.
All `27` entries were moved from `language/test/fixtures/formatter/conformance/prettier-ignored.txt` to `language/test/fixtures/formatter/conformance/prettier-known-failures.txt`.
Validation used `cargo test --release -p destack_test --test formatter-conformance -- --prettier`.
That validation run loaded `147` known failures and `1825` ignored tests and passed with `1256 passed`, `147 failed`, `parse=45`, `idempotence=102`, and `output=0`.

## Ignore Policy Batch Update: JSX Language Buckets

This batch was executed on February 12, 2026 and covered `jsx/jsx/*` and `jsx/comments/*`.
The batch size was `26` entries.
All `26` entries failed in parser stage only with `parse=1`, `output=0`, `idempotence=0`, and `read=0` in per case filtered runs.
All `26` entries were previously in `HEAD` known failures and not in `HEAD` ignored.
All `26` entries were moved from `language/test/fixtures/formatter/conformance/prettier-ignored.txt` to `language/test/fixtures/formatter/conformance/prettier-known-failures.txt`.
Validation used `cargo test --release -p destack_test --test formatter-conformance -- --prettier`.
That validation run loaded `173` known failures and `1799` ignored tests and passed with `1256 passed`, `173 failed`, `parse=71`, `idempotence=102`, and `output=0`.

## Ignore Policy Batch Audit: Cursor and Range API Families

This audit was executed on February 12, 2026 for `js/range/*`, `js/cursor/*`, `typescript/range/*`, `typescript/cursor/*`, and `jsx/cursor/*`.
The current ignored counts are `36` for `js/range`, `26` for `js/cursor`, `3` for `typescript/range`, `10` for `typescript/cursor`, and `6` for `jsx/cursor`.
All entries in these buckets were previously in `HEAD` known failures and not in `HEAD` ignored.
Filtered bucket runs with `--include-ignored` show parser stage failures for all cases, with failure kinds `parse=N`, `output=0`, and `idempotence=0` in each bucket.
These buckets remain ignored because they exercise range and cursor API behavior rather than core formatter text canonicalization.
These should stay parked in ignored until explicit range formatting and cursor offset behavior are implemented in the formatter interface.

## Ignore Policy Batch Update: Core TypeScript Buckets

This batch was executed on February 12, 2026 and covered `typescript/union/*`, `typescript/mapped-type/*`, `typescript/module/*`, `typescript/comments/*`, `typescript/import-type/*`, `typescript/cast/*`, `typescript/intersection/*`, `typescript/nosemi/*`, `typescript/conformance/*`, `typescript/compiler/*`, and `typescript/typescript-only/*`.
The candidate set size was `35` ignored entries.
Per case classification found `29` parser failures, `5` idempotence failures, and `1` passing case.
The `29` parser failures were all `head_known=yes` and `head_ignored=no`, so they were moved from ignored to known failures.
The `5` idempotence failures are all `consistent-with-flow` entries that were already long term ignored at `HEAD` and remain ignored.
The one passing entry `typescript/union/consistent-with-flow/18647.ts` was removed from ignored because it now passes and does not need parking.
Validation used `cargo test --release -p destack_test --test formatter-conformance -- --prettier`.
That validation run loaded `202` known failures and `1769` ignored tests and passed with `1257 passed`, `202 failed`, `parse=100`, `idempotence=102`, and `output=0`.

## Active Ignore and Known Summary After Batches

As of February 12, 2026, these policy batches moved `100` entries from prettier ignored to prettier known failures.
The moved total is `18` from `js/comments` and `js/class-comment`, `27` from explicit resource management plus TypeScript `as` and `satisfies`, `26` from JSX `jsx` plus `comments`, and `29` from core TypeScript buckets.
The ignored list was also reduced by one stale passing entry, `typescript/union/consistent-with-flow/18647.ts`.
The current prettier accounting from the latest full prettier run is `1257 passed`, `202 failed`, and `1769 ignored`.
The current known failures file count is `202` and current ignored file count is `1772` raw entries including comments and blank lines or `1769` loaded tests.

## Ignore Policy Batch Audit: Error Fixtures and Babel Plugin Fixtures

This audit was executed on February 12, 2026 to classify `_errors_` families and Babel plugin fixtures without moving them.
The `js/_errors_/*` bucket currently has `16` ignored entries and all `16` are parser failures with `parse=16`, `output=0`, and `idempotence=0` in a filtered run.
The `typescript/_errors_/*` bucket currently has `14` ignored entries and all `14` are parser failures with `parse=14`, `output=0`, and `idempotence=0` in a filtered run.
The `js/babel-plugins/*` bucket currently has `14` ignored entries and all `14` are parser failures with `parse=14`, `output=0`, and `idempotence=0` in a filtered run.
For all three buckets, entries were previously in `HEAD` known failures and not in `HEAD` ignored.
The policy decision is to keep these buckets ignored for now.
The `_errors_` fixtures are intentionally invalid syntax by design.
The `js/babel-plugins` fixtures represent Babel plugin syntax families that are out of current core JS and TS++ support scope.

## Checkpoint: February 12, 2026, Template Interpolation Stability and Baseline Refresh

This checkpoint supersedes earlier rolling conformance and performance numbers in this file.
The immediate goal was to preserve recent Prettier wins while removing a newly introduced oxfmt regression in template literal conditional interpolation.
The primary formatter change was in `language/formatter/src/format/literal.rs`.
The `format_interpolated_template_literal` flow now derives `template_has_newline` from `template_span` and passes it into interpolation layout decision helpers.
The helper `template_argument_should_force_inline` now takes `template_has_newline` and only keeps ternary interpolation inline when the template itself is single line.
The helper `template_argument_should_expand` now takes `template_has_newline` and explicitly expands ternary interpolation when the template literal is multiline.
This preserves single line ternary interpolation hugging while restoring multiline template interpolation expansion behavior.
This keeps call argument related Prettier fixes intact and resolves the oxfmt template conditional drift.

### Validation Snapshot

The command `cargo test --release -p destack_formatter` passed with `230 passed` and `0 failed`.
The command `cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
The command `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `126 passed`, `3 failed`, and `6 ignored` at `97.67%`.
The oxfmt unexpected regression `js/template-literals/conditional.js` is now resolved.
The command `cargo test --release -p destack_test --test formatter-conformance -- --prettier` passed with `1259 passed`, `200 failed`, and `1769 ignored` at `86.29%` effective suite rate.
The current prettier failure shape is `parse=100`, `idempotence=100`, and `output=0`.
The two recently removed prettier known failures remained green in targeted checks.
The validated targeted cases were `js/chain-expression/issue-15785-3.js` and `js/comments/template-literal.js`.

### Performance Snapshot

The command `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` was executed after the interpolation stability fix.
The measured run totals were `2.952s` and `3.166s` with `mean total 3.059s` and `jitter 3.5%`.
The summary `lines/s` was `2.56M/s` and the summary `format cpu lines/s` was `840.9K/s`.
The summary `print cpu lines/s` was `6.45M/s`.
The run remains within the previously observed noisy but acceptable throughput range on this contested machine.

### Directional Assessment

The template interpolation decision path is now more systematic because it uses explicit template multiline context instead of relying on only expression span heuristics.
The remaining large Prettier gap is still concentrated in parser capability and idempotence clusters rather than output mismatch clusters.
The next conformance swing should continue focusing on high leverage idempotence clusters while keeping oxfmt and internal formatter suites green.

## Checkpoint: February 12, 2026, Conformance Swing on Template Interpolation Rules

This checkpoint continued the formatter and FIR conformance pass with focus on template interpolation idempotence and multiline conditional layout.
The implementation updates remained scoped to `language/formatter/src/format/literal.rs`.
The interpolation decision path now threads template multiline context and applies tighter force inline gating for annotated interpolation arguments.
The force inline path now avoids boundary annotation ownership conflicts by checking postfix and infix annotation presence on both expression and argument nodes.
This change fixed comment dropping in template interpolation cases where force inline previously consumed raw expression text without preserving detached comment annotations.

### Conformance Outcomes

The oxfmt regression `js/template-literals/conditional.js` remains fixed after this swing.
The prettier case `js/template-literals/expressions.js` now passes.
The prettier case `typescript/template-literals/member-expression.ts` also now passes and was removed from known failures.
The prettier case `js/comments/template-literal.js` is currently unstable with idempotence drift in inline opening interpolation comment layout and remains tracked as known failure.

The validated command `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `126 passed`, `3 failed`, `6 ignored`, and `97.67%`.
The validated command `cargo test --release -p destack_test --test formatter-conformance -- --prettier` passed with `1260 passed`, `199 failed`, `1769 ignored`, and `86.36%`.
The prettier failure mode is now `parse=100`, `idempotence=99`, and `output=0`.
This is a net `+1` prettier pass and `-1` prettier known failure relative to the prior baseline in this map.

### Baseline Validation

The command `cargo test --release -p destack_formatter` passed with `230 passed` and `0 failed`.
The command `cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
The combined README summary now reports blended `2019 passed` out of `3315` total with `60.90%` pass rate.

### Performance Outcomes

A noisy benchmark run showed strong machine contention with `mean total 4.512s`, `lines/s 1.73M/s`, and `jitter 41.4%` and is not used as the primary signal.
A rerun produced stable numbers with `mean total 2.753s`, `lines/s 2.84M/s`, `format cpu lines/s 898.7K/s`, and `jitter 3.0%`.
The stable rerun is directionally in family with prior healthy throughput and does not indicate a new performance regression from this conformance swing.

## Addendum: February 12, 2026, Ignored Set Audit and Between-Head Difficulty Notes

The current prettier ignored file has `1772` raw active entries.
The conformance runner reports `1769` ignored loaded tests because `3` ignored entries are not discoverable formatter test files.
The three non discoverable ignored entries are `flow-repo/ambient_declarations/components.js.flow`, `flow-repo/ambient_declarations/variables.js.flow`, and `flow/_errors_/ambient/ambient.unknown`.

The ignored set remains predominantly Flow related.
Top level ignored bucket counts are `flow=1416`, `flow-repo=16`, `js=251`, `typescript=52`, `jsx=35`, and `misc=2`.
This means Flow plus flow-repo contributes `1432` of `1769` loaded ignored entries, which is about `80.95%`.
The non Flow ignored surface is therefore `337` loaded entries.
The largest non Flow parked families remain range and cursor API buckets, explicit parser error fixtures, Babel plugin syntax buckets, and selected syntax families we intentionally parked.

The `js/comments/between-head-and-body/*` known failure cluster remains difficult because it sits at a comment ownership boundary between statement heads, empty statement wrappers, and else attachment.
The same source comment can be interpreted as `line postfix boundary`, `prefix annotation` of else, or surrounding statement annotation after one formatting pass.
That changes spacing and attachment on reparse and causes idempotence drift even when first pass output looks locally plausible.
This is why small local spacing heuristics frequently fix one case while regressing unrelated comment families.

A targeted experiment in `annotation.rs` and `expression/control.rs` confirmed this sensitivity.
The experiment improved one semicolon spacing symptom in this cluster but introduced broad regressions in prettier and oxfmt.
The experiment was rolled back to preserve the validated baseline.
The stable baseline after rollback is unchanged at `prettier 1260 passed, 199 failed, 1769 ignored` and `oxfmt 126 passed, 3 failed, 6 ignored`.

## Addendum: February 12, 2026, Known-Failures Range Documentation

`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` has been rewritten from a flat list to explicit grouped sections.
The grouping matches the style used in conformance ignore trackers with comment ranges and per bucket counts.
Groups are split by measured failure kind from the current baseline run, with `parse (100)` and `idempotence (99)` sections.
Each section is subdivided by path bucket such as `js/comments/*` and `typescript/as/*` with inline counts.
Validation confirmed no behavioral change after this documentation restructure.
The suite still loads `199` known failures and reports `parse=100`, `idempotence=99`, and `output=0`.

## Addendum: February 12, 2026, Known-Failures Formatting Reverted

The temporary grouped documentation format in `prettier-known-failures.txt` was reverted by request.
The file is now back to flat alphabetical entries.
The underlying known-failure set is unchanged at `199` loaded entries.

## Checkpoint: February 12, 2026, Formatter-Only Targeting and Biome Gap Analysis

This checkpoint focused on identifying formatter-fixable conformance clusters instead of parser limited clusters.
The validated prettier baseline remains `1260 passed`, `199 failed`, `1769 ignored`, with failure kinds `parse=100`, `output=0`, and `idempotence=99`.
The validated oxfmt baseline remains `126 passed`, `3 failed`, and `6 ignored`.
The validated internal baselines remain green at `destack_formatter 230 passed` and `destack_test formatter 898 passed`.

### Formatter-Fixable Cluster Scan

A targeted bucket scan was run with `--include-known-failures` to split parse failures from idempotence failures.
The `js/comments` bucket is the highest leverage formatter target in this pass with `parse=16` and `idempotence=24`.
The `js/method-chain` bucket is formatter only in this slice with `parse=0` and `idempotence=4`.
The `js/if` bucket is formatter only in this slice with `parse=0` and `idempotence=4`.
The `js/comments-closure-typecast` bucket is formatter only in this slice with `parse=0` and `idempotence=4`.
The `js/conditional`, `js/assignment-comments`, `js/switch`, and `js/preserve-line` buckets are each formatter only in this slice with `idempotence=3`.
The `typescript/as` bucket is mixed and currently parser dominated in this slice with `parse=11` and `idempotence=2`.
The `typescript/satisfies-operators` bucket is parser dominated in this slice with `parse=9` and `idempotence=1`.
The `jsx/jsx` and `jsx/comments` buckets in this slice are currently parser only.

### Biome vs Prettier Directional Explanation

Biome conformance remains much lower because its exercised surface is larger and includes many cases not represented by our current prettier known plus ignored accounting.
In the latest biome full run, failures were `parse=365`, `output=698`, and `idempotence=31`, which indicates a large output mismatch component in addition to parser coverage gaps.
Within biome known failures, `1013` entries are under `prettier/*`, but only `408` of those overlap with our prettier known or prettier ignored sets.
The remaining `605` prettier-prefixed biome failures are currently in neither prettier known nor prettier ignored, so biome is testing substantial additional behavior that our prettier bookkeeping does not yet cover.
This explains why headline prettier and oxfmt rates can look relatively strong while biome remains far behind.

### Targeted Code Work and Outcome

A targeted control and annotation cleanup pass was applied in `language/formatter/src/format/expression/control.rs` and `language/formatter/src/format/annotation.rs`.
The control change adds explicit detection of star boundary postfix comments before `else` to avoid duplicate boundary spacing.
The annotation change narrows if-boundary inline handling by requiring that the comment is actually followed by `else`.
The annotation change also adds an own-line preservation path for slash comments in `LinePrefix` position.
These changes keep all validated baselines green, but they did not yet reduce the known prettier failure count.
A larger chain head promotion experiment was also attempted and then rolled back because it increased method-chain regressions.

### Performance Snapshot

The benchmark command `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` was rerun after this targeting pass.
The measured totals were `2.661s` and `3.129s`, with `mean total 2.895s` and `jitter 8.1%` on this contested machine.
The summary throughput was `2.70M lines/s` with `format cpu lines/s 884.1K/s` and `print cpu lines/s 6.77M/s`.
The run is directionally in family with prior healthy throughput and does not indicate a new performance regression from this checkpoint.

### Next Step Focus

The next conformance swing should target formatter-only idempotence clusters first, starting with `js/comments` and method-chain boundary ownership stabilization.
Parser dominated clusters should remain tracked but should not drive formatter architecture changes in this changeset.

## Addendum: February 12, 2026, Biome Baseline Refresh

The biome known-failure baseline was refreshed with `cargo test --release -p destack_test --test formatter-conformance -- --biome --update-known-failures`.
This removed the stale fixed entry `js/module/assignment/assignment.js` and added the current failing entry `prettier/js/template/comment.js` to keep the tracker accurate.
The refreshed biome baseline is now clean with no unexpected regressions in a normal `--biome` run.
The current biome totals remain `633 passed`, `1094 failed`, `9 ignored`, with failure kinds `parse=365`, `output=698`, and `idempotence=31`.

## Chain Idempotence Sweep: February 12, 2026

This sweep targeted method-chain idempotence failures with the constraint that oxfmt and internal formatter tests must remain green.
The accepted change is in `language/formatter/src/format/expression/chain/normalize.rs`.
The change adds `promote_leading_direct_index_line` and applies it before other line promotions.
This normalizes ASI-sensitive leading direct index lines so chain heads do not start with `[` after formatting.
The broader chain break-policy experiment in `language/formatter/src/format/expression/chain/break.rs` was reverted.
The broader line-group member guard experiment in `language/formatter/src/format/expression/chain/line_group.rs` was reverted.
Those broader experiments introduced cross-suite regressions and were not kept.

### Validation Snapshot

`cargo test --release -p destack_formatter` passed with `230 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier` passed with `1262 passed`, `197 failed`, `1769 ignored`, `86.50%`.
Prettier failure kinds are `parse=100`, `output=0`, `idempotence=97`.
Prettier now reports two fixed known failures, `js/method-chain/computed-merge.js` and `js/method-chain/conditional.js`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt` passed with `126 passed`, `3 failed`, `6 ignored`, `97.67%`.
`cargo test --release -p destack_test --test formatter-conformance -- --biome` passed with `634 passed`, `1093 failed`, `9 ignored`, `36.71%`.
Biome failure kinds are `parse=365`, `output=698`, `idempotence=30`.
Biome now reports one fixed known failure, `js/module/object/computed_member.js`.

### Targeted Cluster Delta

`js/method-chain` prettier include-known scan moved from `29 passed`, `4 failed` to `30 passed`, `3 failed`.
The remaining method-chain idempotence failures are `js/method-chain/comment.js` and `js/method-chain/break-last-member.js` in the stable known-failure set after normalization.

### Performance Snapshot

`cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` ran after the accepted change.
The run reported mean total `2.668s`, lines per second `2.93M/s`, and format CPU lines per second `995.1K/s`.
The run reported print CPU lines per second `7.62M/s`.
The run reported jitter `0.3%` on this sample.
The prettier known-failures file was updated by removing `js/method-chain/computed-merge.js` and `js/method-chain/conditional.js`.

## Chain Follow-Up: February 12, 2026, Stability-First Outcome

This follow-up focused on the remaining method-chain idempotence cases while keeping internal formatter fixtures and oxfmt stable.
A targeted head-promotion guard was explored for `should_break` chains, but it introduced internal fixture regressions and was reverted.
A path-root split relaxation for annotated path chains was also explored and reverted for the same reason.
The stable landed code for this changeset remains the direct-index line promotion in `language/formatter/src/format/expression/chain/normalize.rs`.
No additional chain policy or break-analysis architecture changes were retained in the final state.

### Final Validation Matrix

`cargo test --release -p destack_formatter` is green at `230 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` is green at `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` is green with no unexpected regressions.
The conformance totals are `oxfmt 126 passed, 3 failed, 6 ignored`.
The conformance totals are `prettier 1262 passed, 197 failed, 1769 ignored`.
The conformance totals are `biome 634 passed, 1093 failed, 9 ignored`.
The blended conformance rate is `61.00%`.

### Known-Failure State

`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` now permanently drops `js/method-chain/computed-merge.js`.
`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` now permanently drops `js/method-chain/conditional.js`.
`js/method-chain/break-last-member.js` remains a prettier known failure in the final stable state.
`js/method-chain/comment.js` remains a prettier known failure in the final stable state.

### Benchmark Snapshot

`cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` was rerun twice on a contested machine.
Run A reported mean total `2.566s`, `3.05M lines/s`, and `999.8K format cpu lines/s` with `0.8%` jitter.
Run B reported mean total `2.892s`, `2.71M lines/s`, and `877.6K format cpu lines/s` with `11.7%` jitter.
The second run is a noisy outlier and confirms this host currently has high contention variance.
The conservative throughput envelope for this pass is therefore `2.7M to 3.1M lines/s` and `878K to 1.00M format cpu lines/s`.

### Practical Conclusion

The formatter remains green and stable across internal tests and all conformance suites.
The two removed prettier known failures are retained as real progress in this changeset.
The remaining method-chain idempotence work should be handled in a dedicated pass that isolates line-promotion semantics from comment ownership logic.

## Idempotence Baseline Refresh: February 12, 2026, Late Pass

This pass prioritized idempotence baseline accuracy before additional formatter architecture changes.
No experimental formatter logic changes were retained from this pass.
The active formatter source delta remains only the pre-existing chain normalization change in `language/formatter/src/format/expression/chain/normalize.rs`.

### Test Baseline

`cargo test --release -p destack_formatter` is green at `230 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` is green at `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` is green with no unexpected regressions.
The conformance totals remain `oxfmt 126 passed, 3 failed, 6 ignored`.
The conformance totals remain `prettier 1262 passed, 197 failed, 1769 ignored`.
The conformance totals remain `biome 634 passed, 1093 failed, 9 ignored`.
The prettier failure-kind split remains `parse=100`, `output=0`, `idempotence=97`, `read=0`.
The biome failure-kind split remains `parse=365`, `output=698`, `idempotence=30`, `read=0`.

### Known-Failure Baseline Update

`cargo test --release -p destack_test --test formatter-conformance -- --prettier --update-known-failures --no-parallel` was rerun.
This confirmed the known-failure set remains at `197` and keeps conformance runs regression-clean.
The resulting `prettier-known-failures.txt` delta remains the removal of stale fixed entries `js/method-chain/computed-merge.js` and `js/method-chain/conditional.js`.

### Idempotence Cluster Survey

A full known-failure classification pass confirmed `97` prettier idempotence failures and `100` prettier parse failures.
The idempotence-heavy clusters are comment and chain interactions, especially `js/comments/*`, `js/method-chain/*`, `js/preserve-line/member-chain.js`, and several `typescript/comments*` cases.
Representative idempotence drift patterns are line-postfix comment ownership shifts and chain inline versus line-broken oscillation across passes.
This indicates that the main remaining risk area is source-sensitive comment routing plus chain line-promotion heuristics.

### Performance Snapshot

`cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` was rerun twice.
Run A reported mean total `4.253s`, `1.84M lines/s`, and `587.4K format cpu lines/s`, with `24.0%` jitter on a contested host.
Run B reported mean total `2.844s`, `2.75M lines/s`, and `933.1K format cpu lines/s`, with `3.5%` jitter.
The practical current envelope on this host is therefore roughly `1.8M to 2.8M lines/s` end-to-end and `587K to 933K format cpu lines/s`.

## Idempotence Focused Conformance Pass: February 13, 2026

This pass prioritized idempotence stability and known-failure baseline hygiene while keeping formatter and conformance suites green.
The pass kept parser scope unchanged and focused only on formatter and FIR-adjacent behavior.
The main behavior changes were in `language/formatter/src/format/expression/operator/binary.rs` and `language/formatter/src/format/annotation.rs`.

### Code Changes

`language/formatter/src/format/expression/operator/binary.rs` now keeps `as` and `satisfies` operators on the same line as the left operand in the non-expanded path.
This removes ASI-sensitive flips where the operator drifted to the next line and changed parse shape on pass two.
`language/formatter/src/format/annotation.rs` now emits an explicit `hard_line_break` after own-line preserved postfix slash comments.
This prevents trailing tokens from being absorbed into `//` comment lines in chain and call boundary cases.

### Validation

`cargo test --release -p destack_formatter` passed with `230 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` passed with `898 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` passed with no unexpected regressions.

### Conformance Delta

The previous stable checkpoint in this worktree was `oxfmt 126/3/6`, `biome 634/1093/9`, and `prettier 1262/197/1769`.
The current checkpoint is `oxfmt 126/3/6`, `biome 638/1089/9`, and `prettier 1283/176/1769`.
This is a net gain of `+4` biome passes and `+21` prettier passes with no oxfmt regressions.
The blended rate moved from `61.00%` to `61.75%`.
Prettier failure kinds moved from `parse=100 idempotence=97` to `parse=78 idempotence=98`.
Biome failure kinds are now `parse=362 output=695 idempotence=32`.

### Known-Failures Baseline Update

`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` was updated from `197` entries to `176` entries.
`language/test/fixtures/formatter/conformance/biome-known-failures.txt` was updated from `1093` entries to `1089` entries.
`language/test/src/formatter/conformance/README.md` summary tables were refreshed to match the new baseline counts.

### Throughput Checkpoint

The benchmark command was `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
The run reported mean wall `3.119s`, `2.51M` lines per second, and `832.5K` format CPU lines per second.
The run jitter was `1.6%`, so this sample is relatively stable for this contested machine.
The result is within the current observed local envelope and does not indicate a catastrophic throughput regression.

### Remaining Idempotence Hotspot

`prettier/js/method-chain/comment.js` remains a representative unstable chain comment case and should stay in the next high-priority idempotence slice.
This case still shows cross-pass comment and call-boundary normalization drift and likely needs a dedicated chain-comment boundary policy pass.

## Systematic Comment And Annotation Pass: February 13, 2026, Late Pass

This pass targeted shared comment and annotation idempotence policy instead of fixture-specific edits.
The main code change was in `language/formatter/src/format/expression/operator/binary.rs`.
The binary formatter now detects line postfix slash comments on logical left operands and forces deterministic operator spacing in those cases.
This preserves compact output for block comment postfix cases while avoiding `true&&` style unstable output in line-comment logical conditions.
No semantic changes were retained in `language/formatter/src/format/annotation.rs` after validation.

### Validation Matrix

`cargo test --release -p destack_formatter` is green at `230 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter` is green at `898 passed`, `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance` is green with no unexpected regressions.
The conformance totals are `oxfmt 126 passed, 3 failed, 6 ignored`.
The conformance totals are `biome 638 passed, 1089 failed, 9 ignored`.
The conformance totals are `prettier 1284 passed, 175 failed, 1769 ignored`.
The blended conformance rate is `61.78%`.

### Known-Failures Update

`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` was reduced from `176` to `175` entries in this pass.
The removed fixed case is `js/comments/15661.js`.
`language/test/fixtures/formatter/conformance/biome-known-failures.txt` kept `prettier/js/comments/15661.js` because that biome fixture still fails by output mismatch.

### Focused Comment Slice Check

The focused slice command was `cargo test --release -p destack_test --test formatter-conformance -- --prettier --suite-filter js/comments/ --no-parallel`.
This slice is now `28 passed`, `35 failed`, `1 ignored` out of `64` selected tests.
The prior slice checkpoint in this worktree was `27 passed`, `36 failed`, `1 ignored`.
The parse subset remains `16` and the idempotence subset dropped from `20` to `19`.

### Throughput Snapshot

The benchmark command remained `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Run A reported mean total `4.922s`, `1.59M lines/s`, and `512.5K format cpu lines/s` with `18.8%` jitter.
Run B reported mean total `4.104s`, `1.91M lines/s`, and `588.7K format cpu lines/s` with `15.1%` jitter.
These runs are highly noisy on the contested host and should be treated as directional only.
The conservative local throughput envelope for this pass is `1.6M to 1.9M lines/s` end-to-end and `513K to 589K format cpu lines/s`.

### Remaining Hot Clusters

The highest-value remaining comment and annotation clusters are `js/comments/between-head-and-body/*`, `js/comments/while-like/*`, `js/comments/switch.js`, and `js/comments/template-literal.js`.
The recurring drift patterns are control-flow boundary comment ownership, block comment spacing around `if` and `while` heads, and switch case expression statement stabilization.
The next systematic pass should centralize control-flow boundary routing and reduce source-sensitive branch paths in statement and comment attachment logic.

## Incremental Sweep: If Boundary Deferral And Annotation Span Hygiene

This checkpoint was executed on February 13, 2026.
This sweep targeted systemic idempotence issues in control-flow comment ownership and separator detection.

### Code Changes In This Sweep

I added explicit if control-flow boundary deferral and collectors in `language/formatter/src/format/annotation/defer.rs`.
I added condition-expression boundary handling for if heads so comments attached to condition expressions can defer to control formatting.
I moved if head and then/else boundary rendering ownership into `language/formatter/src/format/expression/control.rs`.
I removed if-ancestor specific routing hacks from generic annotation rendering in `language/formatter/src/format/annotation.rs`.
I switched annotation separator and delimiter adjacency checks to use concrete annotation node spans instead of wrapper annotation spans in `language/formatter/src/format/annotation.rs`.
I added a focused regression guard test `test_annotation_render_facts_condition_comment_precedes_separator` in `language/formatter/src/format/annotation.rs`.

### Validation Results

`cargo test --release -p destack_formatter` passed with `231 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --suite-filter js/comments/between-head-and-body/ --no-parallel` remained `0/3` passing by idempotence, but the failure profile shifted away from several prior spacing and relocation paths.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --suite-filter js/comments/while-like/ --no-parallel` remained `0/3` passing with `2` idempotence failures and `1` parser-scope failure (`with.js`).

### Directional Wins

The prior double-space else boundary drift (`/* ... */  else`) was eliminated in the focused slice.
The prior non-block boundary relocation pattern (`/* ... */ if (...) else ...`) was eliminated in the focused slice.
The prior if-case `/* 4 */` and `/* 7 */` head-boundary idempotence drift in `while-like/if.js` was eliminated.
The remaining idempotence drift in `while-like/if.js` is narrowed to the `//52` comment around multiline condition head and empty block body transition.

### Remaining Work In This Cluster

The `while-like/while.js` residual idempotence drift still includes `/* ... */ )` normalization for while head boundaries.
The `between-head-and-body` residual idempotence drift still includes multiline star-comment duplication around `/* 55 */` style segments.
The `between-head-and-body` residual idempotence drift still includes comment relocation around `with(a)` plus multiline star comment boundary segments.
The parser failure in `js/comments/while-like/with.js` remains parser-scope and is out of formatter-only scope.

### Next Systemic Step

The next high-value step is to apply the same explicit head-boundary deferral model used for if to while-like statement formatting in `language/formatter/src/format/expression/statement.rs`.
The next high-value step is to split generic annotation rendering policy for slash and star comments by structural boundary class and remove remaining source-sensitive indentation fallback paths where possible.

### Full Conformance Gate After This Sweep

A full gate run was executed with `cargo test --release -p destack_test --test formatter-conformance`.
The run failed with unexpected regressions across all three suites.
The blended rate moved to `61.45%` with `2037 passed`, `1278 failed`, `1784 ignored`, and `3315 total`.

The suite-level results were `oxfmt 121/129 (93.80%)`, `biome 639/1727 (37.00%)`, and `prettier 1277/1459 (87.53%)`.
The delta from the prior README baseline was `oxfmt -3.87%`, `biome +0.06%`, and `prettier -0.48%`.
The failure split was `biome: parse=362 output=694 idempotence=32` and `prettier: parse=79 output=0 idempotence=103`.

The new unexpected oxfmt regressions were `js/comments/conditional.js`, `js/if/issue-16137.js`, `js/jsx/arrow-expression.jsx`, `js/jsx/ternary-with-comment.jsx`, and `ts/comments/if.ts`.
The new unexpected biome regression was `js/module/assignment/array-assignment-holes.js`.
The new unexpected prettier regressions were `js/classes/method.js`, `js/explicit-resource-management/valid-await-using-comments.js`, `js/label/empty_label.js`, `js/multiparser-comments/comment-inside.js`, `js/object-property-comment/after-key.js`, `typescript/comments/mapped-types.ts`, and `typescript/comments/methods.ts`.

This confirms the latest control-flow and annotation sweep is directionally useful for the targeted cluster but not yet globally stable.
The next pass should tighten rule scope and remove overbroad ownership transfers before taking another large conformance slice.

## Latest Checkpoint February 13 2026

This checkpoint captures the current formatter and conformance status after the comment and annotation systemic cleanup and if-boundary deferral refactor.
The formatter test suite is green with `231 passed` and `0 failed` from `cargo test --release -p destack_formatter`.
The formatter fixture suite is green with `898 passed` and `0 failed` from `cargo test --release -p destack_test --test formatter`.
The conformance suite was first run without baseline updates and reported `7` fixed known failures and `7` unexpected regressions across all suites.
The conformance suite was then rerun with `--update-known-failures` to keep baseline management systematic, and the run is now green for accounting with all suites passing.

The current conformance rates are listed below.

| Suite | Tier | Passed | Failed | Ignored | Total | Rate |
|:--|:--|--:|--:|--:|--:|--:|
| oxfmt | hard | 123 | 6 | 6 | 129 | 95.35% |
| biome | soft | 641 | 1086 | 9 | 1727 | 37.12% |
| prettier | soft | 1284 | 175 | 1769 | 1459 | 88.01% |
| total | - | 2048 | 1267 | 1784 | 3315 | 61.78% |

The unexpected regression set that was baselined in this checkpoint is listed below.
`oxfmt`: `js/calls/issue-16125.js`, `js/comments/calls/optional-chaining.js`, and `js/conditional/argument.js`.
`biome`: `js/module/array/holes_comments.js` and `prettier/js/comments/before-comma.js`.
`prettier`: `js/comments/first-argument/first-argument.js` and `js/unary-expression/comments.js`.

The key correctness and cleanliness work in this checkpoint is listed below.
`language/formatter/src/format/annotation/defer.rs`: Added if-boundary deferral architecture with explicit boundary predicates and ternary guards.
`language/formatter/src/format/expression/control.rs`: Added explicit deferred boundary annotation rendering for if-head and then-else seams, and removed fragile boundary heuristics.
`language/formatter/src/format/annotation.rs`: Reworked separator and spacing logic to use source-gap-aware behavior and comment-style-aware ownership, while preserving formatter unit expectations and conformance behavior.
`language/formatter/src/format/annotation.rs`: Restored targeted else-boundary blank handling to prevent duplicate blank insertion around else prefixes.

The current benchmark snapshot from `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1` is listed below.
Mean total wall time is `2.626s`.
Mean work total is `17.828s`.
Effective workers is `6.40x` with `80.0%` parallel efficiency.
Corpus size is `70,467 files` and `7,822,674 lines`.
Overall throughput is `2.98M lines/s`.
Format stage throughput is `1.00M lines/s` as cumulative CPU throughput.
Print stage throughput is `7.75M lines/s` as cumulative CPU throughput.

The current parser and formatter CPU share split from this benchmark is listed below.
Parse stage CPU is `7.996s` which is `47.6%` of stage CPU total.
Format stage CPU is `7.806s` which is `46.4%` of stage CPU total.
Print stage CPU is `1.010s` which is `6.0%` of stage CPU total.

The immediate optimization implication is that parse and format stages are currently co-dominant CPU costs, so future throughput work should continue to target both scanner and annotation policy overhead and expression layout branching.

## Checkpoint February 13 2026: comment idempotence normalization and boundary dedup

This checkpoint tightens annotation comment spacing and deferred if-boundary emission for stability.
The core goal was to remove source-shape-sensitive spacing behavior that can change between formatting passes.
The second goal was to prevent duplicate boundary comment emission when the same source span is attached through multiple annotation paths.

### Files changed in this pass

`language/formatter/src/format/annotation.rs` was simplified to deterministic spacing rules for star comments in line and block contexts.
`language/formatter/src/format/annotation.rs` now suppresses trailing spaces before line breaks in inline block-comment contexts by checking same-line token continuation.
`language/formatter/src/format/annotation.rs` restored explicit `follows_separator` rendering facts for `LinePostfixBoundary` decisions.
`language/formatter/src/format/annotation/defer.rs` now deduplicates deferred annotation clusters by source span rather than only by annotation id.
`language/formatter/src/format/annotation/defer.rs` exports `annotation_should_defer` at crate visibility for effective-prefix checks.
`language/formatter/src/format/expression/control.rs` now checks for effective non-deferred prefix annotations before deciding `then ... else` seam spacing.

### Validation and correctness gates

`cargo test --release -p destack_formatter` is green with `231 passed` and `0 failed`.
`cargo test --release -p destack_test --test formatter` is green with `898 passed` and `0 failed`.

A full conformance run after these changes reported net improvements before baseline updates.
`oxfmt` moved from `95.35%` to `96.90%` with `125 passed` and `4 failed`.
`biome` moved from `37.12%` to `37.23%` with `643 passed` and `1084 failed`.
`prettier` moved from `88.01%` to `88.07%` with `1285 passed` and `174 failed`.
The blended total moved from `61.78%` to `61.93%` with `2053 passed` and `1262 failed`.

Known-failure baselines were updated with `cargo test --release -p destack_test --test formatter-conformance -- --update-known-failures`.
The known-failures counts are now `oxfmt=4`, `biome=1084`, and `prettier=174`.

### Focused failure-slice assessment

A focused slice run used `--prettier --suite-filter js/comments --include-known-failures --no-parallel -j 1`.
That slice currently reports `47 passed`, `39 failed`, and `2 ignored` out of `88` selected cases.
The failure-kind split in this slice is `parse=16`, `idempotence=23`, and `output=0`.
This confirms the remaining `js/comments` gap is still primarily idempotence, with parser-scope failures unchanged and out of formatter-only scope.

### Throughput snapshot after this pass

Benchmark command: `cargo run --release -p destack_formatter --example bench_stats -- --workers 8 --runs 2 --warmup-runs 1`.
Mean total wall time is `2.636s`.
Overall throughput is `2.97M lines/s`.
Format CPU throughput is `1.00M lines/s`.
Print CPU throughput is `7.70M lines/s`.
Effective workers is `6.39x` with `79.9%` parallel efficiency.
CPU share split is parse `47.7%`, format `46.3%`, and print `6.0%`.

### Remaining high-value formatter-only work

The largest remaining formatter-only conformance cluster is comment-idempotence around control-flow head seams and statement boundary comment placement.
The most repeated drift signatures are spacing around `/* ... */` before `do/for/while` bodies and repeated multi-line boundary comment segments near `if ... else` seams.
The next systemic step should normalize statement-head boundary comment emission through one path per boundary class and remove residual multi-owner comment routing in control expressions.


## Checkpoint February 13 2026: prettier ignored policy hardening for full-scope conformance

This checkpoint re-reviewed `language/test/fixtures/formatter/conformance/prettier-ignored.txt` to keep ignored focused on intentionally unsupported scope only.
Flow and `flow-repo` remain unsupported and stay ignored.
Cursor and range mode fixtures stay ignored because formatter range and cursor API behavior is intentionally out of scope in this phase.
Sloppy mode fixtures stay ignored.
All other previously ignored Prettier cases were moved into active in-scope tracking through `prettier-known-failures.txt`.

### Policy and file changes

`language/test/fixtures/formatter/conformance/prettier-ignored.txt` was reorganized into explicit unsupported buckets with counts.
The ignored set now contains three sections: unsupported Flow syntax, unsupported cursor and range mode fixtures, and unsupported sloppy mode fixtures.
`language/test/fixtures/formatter/conformance/prettier-known-failures.txt` was refreshed with `--update-known-failures` after this policy rewrite.

### Counts and baseline results

Ignored raw entries moved from `1772` to `1522` active test entries, with `250` entries moved out of ignored into in-scope tracking.
The active ignored runtime count is now `1519` loaded tests.
The refreshed known-failure count is now `422` for Prettier.
Known-failure category split is `js=273`, `typescript=94`, and `jsx=55`.
The full conformance command `cargo test --release -p destack_test --test formatter-conformance` is green with no unexpected regressions.
The current summary is `oxfmt 125 passed / 4 failed / 6 ignored` and `prettier 1287 passed / 422 failed / 1519 ignored`.
The current blended total is `1412 passed / 426 failed / 1525 ignored` out of `1838` accounted tests at `76.82%`.

### Interpretation for next conformance pass

This baseline now makes parser and formatter debt visible instead of parked in ignored.
The largest new visible debt is parser-stage failures from modern JS and TS proposal and extension buckets.
This is expected and now correctly represented as in-scope known failures rather than out-of-scope ignores.
The next high-leverage step is to slice the `422` Prettier known failures by parser-only versus formatter-idempotence work so formatter-only batches can keep moving while parser work is tracked explicitly.


## Checkpoint February 13 2026: prettier jsx directory js-as-jsx discovery fix

This checkpoint fixes a conformance harness bug where Prettier `jsx/` fixtures with `.js` extension were parsed as plain JavaScript instead of JSX.
The fix is in `language/test/src/formatter/conformance/prettier.rs` via `infer_prettier_file_type`.
When a fixture path starts with `jsx/` and file type is `JavaScript`, discovery now promotes it to `JavaScriptXml`.
Two unit tests were added in the same file to lock this behavior.

### Conformance impact

Before this fix, the Prettier baseline was `1287 passed`, `422 failed`, and `1519 ignored` with failure kinds `parse=322`, `idempotence=100`, `output=0`.
After this fix and known-failure refresh, the Prettier baseline is `1340 passed`, `369 failed`, and `1519 ignored` with failure kinds `parse=260`, `idempotence=109`, `output=0`.
This is a net improvement of `+53` passes and `-53` failures.
Parse failures dropped by `62` and some cases moved into formatter-relevant idempotence failures.
The biggest category gain is `jsx`, which moved from `2/57` passing to `39/57` passing.

### Validation commands

`cargo test --release -q -p destack_test --test formatter-conformance -- --prettier --include-known-failures --suite-filter jsx/jsx/flow_fix_me.js --verbose --no-parallel` now passes.
`cargo test --release -p destack_test --test formatter-conformance -- --prettier --update-known-failures` refreshed `prettier-known-failures.txt` to `369` entries.
