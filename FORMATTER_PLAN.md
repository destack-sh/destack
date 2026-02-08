# Formatter Plan

## Top Rule

JS and TS style behavior is the default formatter baseline for this project.
Destack should follow the same formatter behavior unless DS only syntax requires a narrow divergence.
Do not add JS and TS versus Destack forks unless there is a concrete syntax or semantic requirement.
When divergence is required, keep it minimal, explicit, and covered by fixtures and tests.

This file is private planning material and must not be checked in.
This plan captures the full formatter audit state as of February 6, 2026.

## Scope

The goal is to make Destack formatting excellent for `.ds`, `.ts`, `.tsx`, `.js`, and `.jsx`.
The target style direction is close to `oxfmt`, and therefore close to Biome and Prettier defaults.
The plan also covers TS++ constructs, formatter performance, CLI behavior, and formatter configuration behavior.

## Non-Negotiable Working Notes

These notes are direct constraints for ongoing formatter work.
Default style decisions should follow JS and TS expectations first.
Destack formatting should align with the JS and TS style rules where syntax allows.
Avoid introducing JS and TS versus Destack behavior forks unless the syntax or semantics require a real divergence.
When a divergence is required, keep it narrow, explicit, and covered by tests.
Known acceptable DS exceptions are DS only syntax nodes and the DS leading `&` style edge cases.
Formatter module file names should prefer single-word names, and avoid multipart names unless absolutely required.
`oxfmt` is the closest compliance target.
Biome and Prettier compatibility are aspirational and useful for gap discovery, but not strict release blockers.
We do not need perfect `100%` parity with external formatters.
We should skip changes that are very hacky or disproportionately expensive relative to value.
Parser fixes are tracked in a separate parser changestream.
Formatter work should not expand parser scope unless explicitly requested.
For formatter conformance tracking, temporary exclusions should go to `known-failures` and not `ignored` for active gaps.
Every formatter behavior change should keep formatter unit tests and formatter fixture suites green in the same change stream.

## Primary Audit Targets

`README.md`.
`language/DESIGN.md`.
`language/SPECIFICATION.md`.
`language/formatter/README.md`.
`language/compiler/README.md`.
`language/ast/README.md`.
`language/formatter/src/format/`.
`language/ast/src/tree/`.
`language/parser/src/parse/annotation.rs`.
`language/test/src/formatter/`.
`language/test/fixtures/formatter/`.
`platform/cli/src/common/format.rs`.
`platform/cli/src/command/fmt.rs`.
`platform/cli/src/common/program.rs`.
`platform/daemon/src/command/format.rs`.
`platform/cli/src/tests/fmt.rs`.
`language/workspace/src/config/formatter.rs`.
`language/workspace/src/config/dsconfig.rs`.

## Baseline Results

Pre fix baseline.
`cargo test --release -p destack_formatter` failed to compile due to parser API drift in formatter unit tests.
`cargo test --release -p destack_test --test formatter` passed with `883` tests.
`cargo test --release -p destack_test --test formatter -- roundtrip` reported no tests to run.

Phase 1 current baseline.
`cargo test --release -p destack_formatter` now compiles and passes with `178` tests.
`cargo test --release -p destack_test --test formatter` now passes with `892` tests.
`cargo test --release -p destack_test --test formatter -- roundtrip` now passes with `5` tests.
Roundtrip tests are now executing, and roundtrip idempotence is restored for the current fixture set.

## Progress Log

### 2026-02-08: post-cleanup conformance verification

Validated that the structural cleanup pass did not regress formatter conformance behavior.
Ran `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
Current oxfmt status is `129 / 135` passing with `6` ignored and `0` failed.
The conformance runner remains aligned with parser conformance style, including colorized summary tables and self-updating README behavior.

### 2026-02-08: naming normalization and structural cleanup follow-up

Kept JS and TS style behavior as the default baseline and treated this pass as structure only with no intentional output drift.
Renamed multipart formatter module files to single-word names in the expression and chain subsystems.
Replaced `language/formatter/src/format/source_scan.rs` with `language/formatter/src/format/scan.rs`.
Replaced `language/formatter/src/format/expression/tree.rs` with `language/formatter/src/format/expression/jsx.rs`.
Replaced `language/formatter/src/format/expression/type_binary.rs` with `language/formatter/src/format/expression/binary.rs`.
Replaced `language/formatter/src/format/expression/chain/break_rule.rs` with `language/formatter/src/format/expression/chain/break.rs`.
Removed the generic `engine` bucket and split helpers into focused single-word modules:
`language/formatter/src/format/expression/generic.rs`.
`language/formatter/src/format/expression/scan.rs`.
`language/formatter/src/format/expression/sort.rs`.
Updated formatter module wiring and imports in `language/formatter/src/format/mod.rs`, `language/formatter/src/format/expression/mod.rs`, and `language/formatter/src/format/expression/chain/mod.rs`.
Applied a cleanup pass over formatter branching and iterator logic to reduce convoluted control flow without changing output behavior in these files:
`language/formatter/src/format/annotation.rs`.
`language/formatter/src/format/argument.rs`.
`language/formatter/src/format/declaration.rs`.
`language/formatter/src/format/expression/chain/classify.rs`.
`language/formatter/src/format/expression/chain/format.rs`.
`language/formatter/src/format/expression/control.rs`.
`language/formatter/src/format/expression/core.rs`.
`language/formatter/src/format/expression/operator.rs`.
`language/formatter/src/format/expression/primary.rs`.
Validation is green:
`cargo fmt --all`.
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.

### 2026-02-08: oxfmt burndown pass, template interpolation and declarator width thresholds

Kept JS and TS style behavior as the default baseline and only added formatter logic that applies across JS, TS, and DS where syntax overlaps.
Added class generic heritage detection helpers in `language/formatter/src/format/expression/core.rs` and used them to improve declarator break preference for generic class heritage assignment patterns.
Adjusted declarator long value detection from `>` to `>=` width threshold and refined string and template declarator break preference to favor break-after-operator formatting when the RHS reaches the width boundary.
Updated template interpolation behavior in `language/formatter/src/format/literal.rs`:
single-line ternary interpolations can stay inline via raw interpolation source emission;
multiline template spans force expanded interpolation formatting;
newline detection now considers both expression spans and argument spans.
Adjusted nested type conditional indentation in template interpolation contexts in `language/formatter/src/format/expression/primary.rs`.
Removed fixed oxfmt cases from known failures in `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`:
`js/template-literals/conditional.js`,
`ts/template-literal-type/issue-16136.ts`,
`js/unicode/emoji-sequences.js`.
Updated internal formatter transform expectations to match current behavior:
`language/test/fixtures/formatter/transform/declarations/types-advanced.md`,
`language/test/fixtures/formatter/transform/literals/strings.md`.
Current oxfmt status is `123 / 135` passing, with `6` known failures and `6` ignored.
Remaining known failures:
`js/ignore/expression-statement.js`,
`js/ignore/oxfmt.js`,
`js/quote-props/with-clause.js`,
`ts/class/issue-16259.ts`,
`ts/comments/union.ts`,
`ts/parameters/object-pattern.ts`.
Validation is green:
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt`.
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.

### 2026-02-08: directive path unification and heritage clause alignment pass

Removed the legacy previous-line raw scan fallback from `language/formatter/src/format/directive.rs` so formatter ignore detection uses a single annotation plus token path.
Kept ignore range raw span emission behavior stable by restoring trailing newline trimming in `write_ignored_span`, which preserved internal formatter fixture expectations for range comments.
Adjusted class and interface heritage clause layout in `language/formatter/src/format/declaration.rs` so long `extends` and `implements` clauses can break before the clause keyword and keep the first super type on the clause line.
Updated internal transform fixture expectation for long class `implements` wrapping in `language/test/fixtures/formatter/transform/declarations/classes.md`.
Validated green formatter suites:
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
Validated oxfmt conformance stability at `117 / 135` passing with `12` known failures and `6` ignored:
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
Attempted chain-head promotion restrictions for comment-boundary member chains, then reverted those experiments after they introduced regressions in non-known-failure fixtures.

### 2026-02-08: chain module cleanup, folder split, and singular subfiles

Kept JS and TS style behavior as the default baseline and treated this pass as structure only with no intentional output drift.
Replaced monolithic `language/formatter/src/format/expression/chain.rs` with a folder module:
`language/formatter/src/format/expression/chain/mod.rs`.
Split chain logic into singular subfiles with cohesive responsibilities:
`language/formatter/src/format/expression/chain/base.rs`.
`language/formatter/src/format/expression/chain/classify.rs`.
`language/formatter/src/format/expression/chain/length.rs`.
`language/formatter/src/format/expression/chain/break_rule.rs`.
`language/formatter/src/format/expression/chain/format.rs`.
Preserved existing function behavior and public surfaces used by sibling formatter modules via module re-exports in `chain/mod.rs`.
Validation is green:
`cargo fmt -p destack_formatter`.
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
`cargo clippy --release -p destack_formatter`.
oxfmt baseline is unchanged at `117 / 135` passing with `12` known failures and `6` ignored.

### 2026-02-08: core decomposition pass, primary and operator cluster extraction

Kept JS and TS style behavior as the default baseline and treated this pass as structure only with no intentional output drift.
Extracted primary expression formatting arms from `language/formatter/src/format/expression/core.rs` into `language/formatter/src/format/expression/primary.rs`.
Extracted operator and chain expression formatting arms from `language/formatter/src/format/expression/core.rs` into `language/formatter/src/format/expression/operator.rs`.
Rewired `format_expression` in `language/formatter/src/format/expression/core.rs` into a thin dispatcher that delegates in this order: statement, primary, operator.
Kept singular file naming for new modules and retained the same unsupported expression fallback behavior in `core.rs`.
Current expression layout snapshot:
`language/formatter/src/format/expression/core.rs` is now `515` lines.
`language/formatter/src/format/expression/primary.rs` is `655` lines.
`language/formatter/src/format/expression/operator.rs` is `1164` lines.
Validation is green:
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
oxfmt baseline is unchanged at `117 / 135` passing with `12` known failures and `6` ignored.
Quality check:
`cargo clippy --release -p destack_formatter` passes with existing non blocker warnings, including some pre existing warnings and a few simplification suggestions in extracted modules.

### 2026-02-08: expression decomposition pass, control/member/object/test extraction

Kept JS and TS style behavior as the default baseline and treated this pass as structure-only with no intentional output drift.
Split control flow helpers out of `language/formatter/src/format/expression.rs` into `language/formatter/src/format/expression/control.rs`.
Split member, postfix, and binary flatten helpers out of `language/formatter/src/format/expression.rs` into `language/formatter/src/format/expression/member.rs`.
Split array boundary and object literal policy helpers out of `language/formatter/src/format/expression.rs` into `language/formatter/src/format/expression/object.rs`.
Moved the large expression unit test module out of production code into `language/formatter/src/format/expression/tests.rs`.
Kept the parent expression orchestrator file as a thin coordination layer with stable exports and submodule wiring.
Naming note: prefer `analysis` or `layout` for heuristic modules and avoid `metrics` naming.
Validation is green:
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
oxfmt baseline is unchanged at `117 / 135` passing with `12` known failures and `6` ignored.

### 2026-02-08: module layout normalization, expression folder module

Moved the expression module root from `language/formatter/src/format/expression.rs` to `language/formatter/src/format/expression/mod.rs`.
Added `language/formatter/src/format/expression/engine.rs` as the internal shared-helper module.
Kept `language/formatter/src/format/expression/mod.rs` as the small module hub with submodule wiring and re-exports.
Removed the top-level `language/formatter/src/format/expression.rs` file to avoid split-root module layout.
Validation is green:
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
oxfmt baseline is unchanged at `117 / 135` passing with `12` known failures and `6` ignored.

### 2026-02-08: core decomposition pass, statement cluster extraction

Kept JS and TS style behavior as the default baseline and treated this pass as structure-only with no intentional output drift.
Extracted statement-like expression formatting arms from `language/formatter/src/format/expression/core.rs` into `language/formatter/src/format/expression/statement.rs`.
Rewired `format_expression` in `language/formatter/src/format/expression/core.rs` to delegate to `format_statement_expression` first, then continue with non-statement expression formatting.
Used singular file naming for the new module (`statement.rs`) and will keep singular names for future expression submodule splits.
Validation is green:
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
oxfmt baseline is unchanged at `117 / 135` passing with `12` known failures and `6` ignored.

### 2026-02-08: structural cleanup pass, chain extraction and dedup

Kept JS and TS style behavior as the default baseline and treated cleanup as structure-only with no intentional output drift.
Split the chain subsystem out of `language/formatter/src/format/expression.rs` into `language/formatter/src/format/expression/chain.rs`.
Split the ternary and parenthesized-boundary subsystem out of `language/formatter/src/format/expression.rs` into `language/formatter/src/format/expression/ternary.rs`.
Split the call and instantiation subsystem out of `language/formatter/src/format/expression.rs` into `language/formatter/src/format/expression/call.rs`.
Kept the external expression formatter API stable via re-exports from `language/formatter/src/format/expression.rs`.
Kept classifier helpers in `language/formatter/src/format/expression/classify.rs` and wired shared imports through the parent module.
Deduplicated method and field formatting paths in `language/formatter/src/format/property.rs` with shared `format_method_like` and `format_field_like` helpers.
Kept shared source scanning and signature policy modules in use:
`language/formatter/src/format/source_scan.rs`.
`language/formatter/src/format/signature.rs`.
Post-refactor size snapshot:
`language/formatter/src/format/expression.rs` is now `8677` lines.
`language/formatter/src/format/expression/chain.rs` is `2967` lines.
`language/formatter/src/format/expression/ternary.rs` is `790` lines.
`language/formatter/src/format/expression/call.rs` is `1318` lines.
`language/formatter/src/format/expression/classify.rs` is `271` lines.
Validation is green:
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.
oxfmt baseline is unchanged at `117 / 135` passing with `12` known failures and `6` ignored.

### 2026-02-07: TS parameter and heritage normalization pass

Kept JS and TS behavior as the baseline and removed formatter regressions caused by overly broad single-parameter expansion.
Generalized static parameter detection across declarations, properties, and members so TS generic constraints now format with `extends` instead of DS style `:`.
Reworked function signature parameter list formatting to use shared list behavior, while preserving compact single multiline pattern wrappers where oxfmt expects `fn({ ... })` instead of `fn(\n  { ... },\n)`.
Replaced multiline heritage clause parenthesis wrappers with JS and TS style multiline `extends` and `implements` list formatting.
Expanded type context object literal handling to preserve multiline type object layout from source in TS contexts, which restored type literal semicolon behavior in affected conformance fixtures.
Relaxed trivial expression detection for type binary casts and static type arguments so callback plus cast call arguments do not over-break.
Scoped parameter object expansion heuristics away from JS files to avoid JS assignment regressions while preserving TS readability wins.
Updated transform fixture expectations for class long implements line breaking and advanced destructuring cases to match current stable formatter output.
Updated `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` to `37` known failures after removing three newly passing cases.
Current oxfmt result is `92 / 135` passing, with `0` parse failures and `37` output failures.
Current local formatter suites are green with `cargo test --release -p destack_formatter` and `cargo test --release -p destack_test --test formatter`.

### 2026-02-07: oxfmt single-member type wrapper cleanup pass

Kept JS and TS style behavior as the default baseline and avoided adding DS specific forks for shared type formatting rules.
Fixed parenthesized type wrapper dropping so single member leading union and leading intersection wrappers in parenthesized array element types now collapse to bare element types.
Generalized leading grouping operator source detection to handle both span shapes seen across parser modes, including sources with and without the outer parenthesis in the captured span.
Added `TestFormatter::parse_with_file_type` so formatter unit tests can run parser and formatter logic in explicit file type modes, including TypeScript mode.
Updated the single member leading intersection test to run in TypeScript mode with TypeScript formatter options so the unit test now reflects conformance mode behavior.
Validated green formatter and formatter suite runs after formatting changes.
Current oxfmt result is `88 / 135` passing with `41` known failures and `0` parse failures.

### 2026-02-07: oxfmt assignment and single-parameter formatting pass

Kept JS and TS style behavior as the default baseline and avoided DS-only branching for shared syntax.
Added precedence-safe postfix base formatting in shared expression paths, including chain bases and member/index receivers.
Added precedence-aware binary operand parenthesization in the shared binary operand formatter.
Adjusted declarator best-fitting candidate order so long breakable right-hand sides prefer breaking after `=` before expanding destructuring headers.
Fixed shared list formatting so empty lists never emit trailing separators when groups break.
Refined single-parameter function signature formatting to avoid list mode over-breaks and trailing comma artifacts for one parameter signatures.
Updated local transform expectation for complex destructuring function parameters to match new stable formatter output.
Removed fixed oxfmt cases from known failures: `js/assignments/issue-16089.js` and `js/assignments/issue-16704.js`.
Current oxfmt result is `86 / 133` passing with `47` known failures, `8` parse failures, and `39` output failures.
Current local formatter suites are green: `cargo test --release -p destack_formatter` and `cargo test --release -p destack_test --test formatter`.

### 2026-02-07: oxfmt union and type alias indentation pass

Kept JS and TS baseline behavior as default and removed risky DS specific divergence for parenthesized type alias wrapper dropping.
Refined type alias declaration formatting so comments before type values break after `=` with correct indentation.
Adjusted leading pipe union output to avoid double indentation when union expressions already carry prefix annotations.
Added boundary line comment handling for leading pipe unions so comments between union operands preserve `|` placement before the next operand.
Restored type object expansion heuristics only for complex static type argument objects to recover `ts/assignments/complex-type-arguments.ts`.
Fixed mixed type operator precedence regressions by changing type chain flattening to flatten only identical operators.
Added grouping parenthesis support for mixed type binary operands in the shared binary output path.
Updated `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` to `49` failures after removing four newly passing tests.
Current oxfmt result is `84 / 133` passing with `12` parse failures and `37` output failures.
Current `ts/union` slice result is `4 / 6` passing with one parser failure and one remaining output failure (`ts/comments/union.ts`).
Core formatter suites are green: `cargo test --release -p destack_formatter` and `cargo test --release -p destack_test --test formatter`.

### 2026-02-07: oxfmt new-callee and empty-if block alignment pass

Kept JS and TS style behavior as the baseline and avoided introducing DS specific rule forks.
Refactored `new` callee formatting to drop redundant parentheses for simple member path callees and to preserve grouping for call based member callees.
Added member object parenthesis unwrapping for parenthesized call objects in direct member access output paths.
Aligned empty `if` and `else` block formatting with oxfmt non collapsible behavior so empty control flow blocks stay expanded.
Updated formatter unit tests for new callee grouping behavior and empty `if` block behavior.
Updated local formatter transform fixture expectations in `language/test/fixtures/formatter/transform/expressions/new.md` for `new (Foo.bar)(value)` normalization.
Removed `ts/new-expression/issue-16203.ts` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `80 / 133` passed with `53` known failures, `12` parse failures, and `41` output failures.
Current local formatter suite result is `823 / 823` passing, with `73` existing skips unchanged.

### 2026-02-07: oxfmt empty optional call boundary comment pass

Kept JS and TS style rules as the default baseline and removed formatter only chain special cases that were forcing breaks for deferred optional call boundary comments.
Refactored call output so direct call formatting and chain call formatting share the same deferred empty call boundary comment rendering path.
Expanded deferred boundary comment handling to cover slash style `BlockPostfix` comments in empty call argument slots, plus line boundary optional call comments.
Adjusted chain break heuristics so deferred empty call boundary comments do not force chain line breaks or non inline annotation classification.
Added formatter transform fixtures for optional call boundary line comments, optional inline empty argument comments, and empty call line comment argument cases.
Removed `js/calls/issue-16125.js` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `68 / 133` passed with `65` known failures, `12` parse failures, and `53` output failures.
Current local formatter suite result is `896 / 896` passing.

### 2026-02-07: oxfmt call-chain template literal pass

Kept JS and TS style behavior as the default baseline and continued narrowing formatter output gaps without DS specific forks.
Added a targeted chain line grouping rule for template literal call segments so chained `.toMatchInlineSnapshot(...)` style calls can stay on one line when they should.
Added inline handling for delimiter bounded block postfix star comments in annotation formatting to avoid forced hard line breaks for `(/* ... */)` style comment slots.
Removed `js/calls/template-literal-argument.js` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `67 / 133` passed with `66` known failures, `12` parse failures, and `54` output failures.

### 2026-02-07: oxfmt chain head and annotation punctuation pass

Kept JS and TS baseline behavior as default and only narrowed DS divergence where syntax requires it.
Refined chain head promotion so first direct calls stay attached by default, while non head callback chains with an argumented first call can still break early.
Stabilized inline comment punctuation handling around calls and members by expanding separator aware spacing rules and inline block comment delimiter detection.
Improved parenthesized expression layout for comment and newline leading trivia with safer cast left parentheses dropping.
Updated formatter transform fixtures for TSX comment containers and chain head expectations to keep local suites aligned with new behavior.
Removed `ts/conditional-type/nested-test.ts` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `66 / 133` passed with `67` known failures, `12` parse failures, and `55` output failures.

### 2026-02-07: TS union wrapper and field modifier normalization pass

Kept the JS and TS baseline rule explicit while narrowing new behavior to source driven TS union wrapper cleanup cases only.
Added a targeted parenthesized type unwrap rule for union leading source shapes like `| (...)` so oxfmt `ts/union/parenthesis.ts` now passes without broad TS conditional or intersection regressions.
Fixed member field modifier postfix formatting to preserve definite assignment `!` for non accessor fields while keeping accessor forms aligned with TS expectations.
Validated that formatter unit tests and formatter transform suites remain green after the narrowing.
Updated `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` by removing `ts/union/parenthesis.ts`.
Current oxfmt result is `63 / 133` passing with `70` known failures, `12` parse failures, and `58` output failures.

### 2026-02-07: oxfmt js comments stabilization pass

Completed a larger formatter pass for JS and TS style alignment around empty block preservation, JSX paired tag closing layout, logical chain operator placement, and call argument trailing comma placement before line comments.
Aligned statement semicolon behavior for ternary expressions by narrowing AST statement-like classification so ternaries are not treated as top-level control flow statements.
Kept JS and TS default behavior as the baseline and updated local formatter unit and transform fixtures where legacy expectations diverged from the new JS and TS aligned rules.
Removed `js/comments/computed-member.js` and `js/comments/jsx.jsx` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current focused oxfmt `js/comments` result is `8 / 15` passed with `7` failures, where `2` are parser failures and `5` are formatter output gaps.

### 2026-02-06: oxfmt assignment and comment cluster pass

Completed a rule level formatter pass for assignment and computed index behavior, without fixture specific hacks.
Aligned computed index assignment formatting with oxfmt by parenthesizing assignment index expressions.
Aligned declaration and assignment operator break behavior for prefix and between comments in JS and TS cases.
Aligned multiline object default behavior in multiline pattern defaults by detecting pattern field default object context.
Updated local formatter transform expectations for chain assignment break behavior to match the new JS and TS baseline direction.
Removed five passing cases from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `47 / 133` passed with `86` known failures, `12` parse failures, and `74` output failures.

### 2026-02-06: oxfmt member-chain cluster pass

Completed a focused pass on member-chain formatting for JS and TS style parity.
Fixed semicolon emission for anonymous lambda expression statements in block formatting.
Refined ternary branch indentation so multiline chained call branches match oxfmt style.
Aligned local formatter transform expectations for ternary branch indentation in TS fixtures.
Removed two passing cases from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `49 / 133` passed with `84` known failures, `12` parse failures, and `72` output failures.

### 2026-02-06: oxfmt directive comment case pass

Completed a targeted comment formatting fix for redundant parenthesized lambda statements.
Parenthesized top level lambda statements now drop redundant parentheses when safe in statement context.
Kept annotation safety by preserving parentheses when annotations are attached to the parenthesized wrapper.
Removed one passing case from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `50 / 133` passed with `83` known failures, `12` parse failures, and `71` output failures.

### 2026-02-06: lambda annotation deferral and comment normalization pass

Added line prefix annotation split helpers for before and after anchor starts so lambda wrapper comments can be deferred around `=>` more precisely.
Adjusted lambda declaration and expression formatting to defer only the relevant line prefix annotation subsets while keeping JS and TS default behavior as the baseline.
Normalized slash comment and doc rendering so empty `//` lines and compact inline block comments are preserved reliably.
Updated formatter unit expectations for empty doc comments on lambda arrows to match current JS and TS aligned placement behavior.
Current oxfmt result remains `51 / 133` passed with `82` known failures, `12` parse failures, and `70` output failures.

### 2026-02-06: oxfmt arrow comment case pass

Refactored lambda comment deferral paths so line prefix annotations on statement wrappers, expression wrappers, and argument wrappers converge at the lambda arrow site.
Adjusted call hugging and statement list prefix emission to avoid duplicate pre lambda comment emission in JS and TS paths.
Fixed nested lambda in call argument indentation behavior for commented arrow chains to match the oxfmt snapshot for `js/comments/arrow.js`.
Removed `js/comments/arrow.js` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
Current oxfmt result is `52 / 133` passed with `81` known failures, `12` parse failures, and `69` output failures.

## Coverage Snapshot

There are `59` formatter fixture files under `language/test/fixtures/formatter`.
There are `883` transform test headings (`### ...`) in transform fixtures.
Fixture category case counts are `comments=9`, `declarations=205`, `expressions=301`, `imports=17`, `literals=183`, `statements=127`, and `tsx=41`.
Transform language input block counts are `ds=732`, `ts=112`, and `tsx=42`.
There are currently zero `js` and zero `jsx` transform fixture inputs.
Roundtrip fixture files currently contain only `.ds` coverage.

## High Priority Findings

### F1: Roundtrip formatter tests are not discovered

`language/test/src/formatter/runner.rs` discovers roundtrip files from `fixtures/formatter` root.
`discover_test_files` is non recursive, and roundtrip files live in `fixtures/formatter/roundtrip`.
This silently disables roundtrip coverage in practice.

Evidence files.
`language/test/src/formatter/runner.rs`.
`language/test/src/harness/discover.rs`.
`language/test/fixtures/formatter/roundtrip/`.

### F2: Formatter crate tests are compile broken

`language/formatter/src/format/property.rs` tests call `eat_struct_or_class` with two arguments.
Parser now requires a third `allow_anonymous_class: bool` argument.
This blocks formatter crate level unit tests entirely.

Evidence files.
`language/formatter/src/format/property.rs`.
`language/parser/src/parse/struct.rs`.

### F3: CLI formatter does not scan TS and JS files by default

`platform/daemon/src/command/format.rs` only includes `Destack` and `Json` in `FORMATTABLE_TYPES`.
This contradicts formatter crate support and user expectation for TS and TSX formatting.

Evidence files.
`platform/daemon/src/command/format.rs`.
`language/formatter/README.md`.

### F4: CLI formatter config loading is partial and divergent

CLI daemon formatter path loads a custom minimal `dsconfig` struct with only `line_ending`, `indent_style`, `indent_width`, and `line_width`.
It ignores existing workspace formatter options such as quote style, trailing commas, bracket spacing, arrow parentheses, quote props, JSX options, and import sorting.
It also rebuilds options from default values rather than merging over `Program` formatter options.

Evidence files.
`platform/daemon/src/command/format.rs`.
`language/workspace/src/config/formatter.rs`.
`language/workspace/src/config/dsconfig.rs`.

### F5: CLI per file error handling is polluted by global diagnostics

Formatter diagnostics are merged into `program.diagnostics` and checked globally for each file.
A prior file parse error can incorrectly fail later files.
Per file formatting should use per file diagnostic collections.

Evidence files.
`platform/daemon/src/command/format.rs`.

### F6: CLI formatting traversal has no ignore policy and no deterministic ordering

Recursive directory walking does not filter directories like `node_modules`, `target`, `.git`, `dist`, and generated cache directories.
Collected files are not explicitly sorted before formatting, which can cause nondeterministic output order.

Evidence files.
`platform/daemon/src/command/format.rs`.

### F7: CLI format tests are minimal and only validate JSON

Current CLI `fmt` tests only validate one JSON formatting path.
There is no CLI test coverage for `.ds`, `.ts`, `.tsx`, `--check`, organized imports, formatter options, ignored files, or report payload shape.

Evidence files.
`platform/cli/src/tests/fmt.rs`.

## Medium Priority Findings

### F8: JS and JSX fixture parity is missing

Transform fixtures exercise DS, TS, and TSX heavily, but not JS and JSX directly.
Given parser and formatter support JS and JSX, this is a practical blind spot.

Evidence files.
`language/test/fixtures/formatter/transform/`.

### F9: Some AST level TS surface is still marked incomplete

Triple slash reference directives are explicitly marked incomplete.
Pattern assignment expression support without `let` is marked incomplete.
These are known parser and AST gaps that impact true TS parity.

Evidence files.
`language/ast/src/tree/expression.rs`.

### F10: No dedicated formatter benchmark target exists

There is parser and compiler bench infra, but no formatter benchmark in `language/formatter`.
Performance work currently has no stable baseline, no regression gate, and no CI measurable target.

Evidence files.
`language/formatter/Cargo.toml`.
`language/justfile`.
`Cargo.toml` workspace bench dependencies.

## Architecture and Strengths to Preserve

Annotation handling is rich and intentionally CST-like.
Annotation positions include block and line prefix, infix, postfix, and postfix boundary.
Parser annotation attachment logic is sophisticated and should remain a first class constraint when refactoring formatter internals.
AST and formatter variant coverage is currently structurally complete for `Expression`, `Declaration`, and `Pattern` variant matches.

Evidence files.
`language/ast/src/tree/annotation.rs`.
`language/ast/src/tree/tree.rs`.
`language/parser/src/parse/annotation.rs`.
`language/formatter/src/format/annotation.rs`.
`language/formatter/src/format/expression.rs`.

## TS, TSX, and TS++ Surface Checklist

### TypeScript and TSX coverage checklist

- [ ] Add transform fixtures for `.js` syntax behavior.
- [ ] Add transform fixtures for `.jsx` behavior.
- [ ] Add `.d.ts` fixture set for declaration file specific behavior.
- [ ] Add triple slash directive fixtures once parser support exists.
- [ ] Add parity fixtures for tricky TS parser ambiguities in TSX.
- [ ] Add parity fixtures for import attributes, import equals, export equals, and namespace exports.
- [ ] Add parity fixtures for sequence expressions in all contexts.
- [ ] Add parity fixtures for decorators across declarations, members, params, and match arms.
- [ ] Add parity fixtures for private fields and private member chains.
- [ ] Add parity fixtures for type mapped, infer, type import, and asserts predicate forms.

### TS++ specific coverage checklist

- [ ] Extend fixtures for ownership operators in value and type positions.
- [ ] Extend fixtures for `comptime` expression and block behavior in nested contexts.
- [ ] Extend fixtures for `match` with advanced pattern combinations.
- [ ] Extend fixtures for `extension` declarations with complex generics and where clauses.
- [ ] Extend fixtures for tree literal plus TS++ expression interactions.
- [ ] Extend fixtures for annotation and directive behavior on TS++ constructs.

## CLI and Config Behavior Checklist

### CLI UX checklist

- [ ] Support formatting default scan for DS, TS, TSX, JS, JSX, D.TS, JSON, and optionally TOML and YAML.
- [ ] Add ignore behavior aligned with expected tooling defaults.
- [ ] Sort target file list deterministically.
- [ ] Keep `--check` semantics predictable with stable output and exit codes.
- [ ] Ensure report payload always includes complete summary fields.

### Config wiring checklist

- [ ] Remove custom minimal dsconfig formatter parser in daemon format path.
- [ ] Reuse workspace resolved formatter options.
- [ ] Implement explicit precedence order: CLI flags over workspace config over defaults.
- [ ] Ensure all formatter options are honored consistently for CLI and test harness.
- [ ] Add tests for option precedence and serialization behavior.

## Performance Plan

### Metrics to establish

- [ ] Throughput in bytes per second for DS, TS, and TSX corpora.
- [ ] P50, P95, and worst file latency.
- [ ] Allocations and peak memory for representative large files.
- [ ] Cost of organize imports and comment heavy formatting.

### Benchmark implementation checklist

- [ ] Create formatter bench target in `language/formatter/benches/` using criterion.
- [ ] Add mixed corpus fixtures under `language/test/fixtures/formatter/bench/`.
- [ ] Include cold parse plus format and warm format only benchmark modes.
- [ ] Add a CLI level benchmark command for end to end file traversal path.
- [ ] Add regression threshold checks in CI for critical benchmark cases.

## Workstreams

## 1) Test Infrastructure First

Goal.
Restore broken formatter test compilation and ensure roundtrip tests actually execute.

Checklist.
- [x] Fix parser API drift in formatter unit tests.
- [x] Fix roundtrip discovery path.
- [x] Verify `cargo test --release -p destack_formatter` compiles and runs.
- [x] Verify `cargo test --release -p destack_test --test formatter -- roundtrip` executes real tests.
- [x] Add regression guard for roundtrip test discovery.
- [x] Fix newly visible roundtrip failures in `formatter-0003.ds` and `formatter-0004.ds`.
- [x] Fix existing formatter unit failures in redundant const borrow and pointer tests.
- [x] Fix decorator parenthesis non idempotence and add a dedicated unit regression test.

Primary files.
`language/formatter/src/format/property.rs`.
`language/test/src/formatter/runner.rs`.
`language/test/src/formatter/roundtrip.rs`.
`language/test/fixtures/formatter/roundtrip/`.
`language/formatter/src/format/expression.rs`.
`language/formatter/src/format/annotation.rs`.

## 2) CLI Formatting Correctness and UX

Goal.
Make `destack fmt` behavior align with modern formatter expectations.

Checklist.
- [x] Expand default formattable file types.
- [x] Implement ignore directories and optionally ignore file support.
- [x] Sort traversal order deterministically.
- [x] Decouple per file diagnostics from global diagnostics.
- [x] Harmonize changed and error reporting across text and JSON modes.
- [x] Add CLI tests for DS and TS paths, check mode, and option precedence.

Phase 2 progress notes.
- [x] Added CLI tests for DS and TS paths and for check mode.
- [x] Added CLI test coverage for default scan ignores (`node_modules`).
- [x] Added CLI regression test for mixed valid and invalid files in one invocation.
- [x] Added option precedence coverage (CLI flags versus dsconfig) for quote style behavior.

Primary files.
`platform/daemon/src/command/format.rs`.
`platform/cli/src/command/fmt.rs`.
`platform/cli/src/common/format.rs`.
`platform/cli/src/tests/fmt.rs`.
`platform/cli/src/common/program.rs`.

## 3) Unified Config Wiring

Goal.
Ensure formatter options are resolved once and applied consistently.

Checklist.
- [x] Route daemon formatter option loading through workspace config APIs.
- [x] Remove local duplicated formatter option deserialization in daemon.
- [x] Preserve all option fields and aliases defined in workspace config.
- [x] Add tests for config merge and CLI override precedence.

Phase 3 progress notes.
- [x] Daemon formatter now resolves dsconfig via resolver and uses normalized workspace formatter options.
- [x] Local minimal dsconfig formatter parsing structs were removed from daemon format path.
- [x] CLI override precedence is enforced over dsconfig values when CLI values differ from defaults.
- [ ] Consider adding explicit CLI field presence tracking to make precedence exact for default-valued flags.

Primary files.
`platform/daemon/src/command/format.rs`.
`language/workspace/src/config/formatter.rs`.
`language/workspace/src/config/dsconfig.rs`.
`platform/cli/src/common/program.rs`.

## 4) Parity Matrix Expansion

Goal.
Close TS and TSX behavior gaps against `oxfmt` and Biome style expectations.

Decision policy.
Prefer JS and TS style parity decisions by default, and treat Destack as aligned unless there is a strong reason not to.
Do not add broad language specific forks just to preserve historical Destack style differences.
Document any unavoidable divergence with a short rationale and a targeted fixture.

Checklist.
- [x] Add formatter local conformance suite scaffold under formatter test harness.
- [x] Add formatter conformance fixture root and seed smoke fixtures.
- [ ] Build explicit feature matrix and map every feature to fixture coverage.
- [ ] Add missing JS and JSX fixtures.
- [ ] Add D.TS specific fixture pack.
- [ ] Add known tricky syntax parity fixtures and expected outputs.
- [ ] Keep TS++ behavior documented and tested with clear style decisions.

Phase 4 progress notes.
- [x] Added `language/test/src/formatter/conformance.rs` with recursive discovery of `conformance/**/input.*` fixtures.
- [x] Conformance tests now support expected output checks via optional `expected.*` files and idempotence checks when expected output is absent.
- [x] Formatter suite runner now integrates transform, roundtrip, and conformance tests in one harness.
- [x] Added initial smoke cases at `language/test/fixtures/formatter/conformance/smoke/`.
- [x] Added fetch scripts for Biome, Prettier, and oxfmt sources under formatter conformance staging.
- [x] Added dedicated external suite harness (`formatter-conformance`) with per suite selection, known and ignored files, colored summary output, and README auto update.
- [x] Imported and baselined staged external suite corpora with initial known failures.
- [x] Rebased formatter fixture expectations to JS and TS aligned defaults for empty blocks and member terminators.
- [ ] Improve parity over the current baseline (47.30% blended pass rate) by reducing known failures.

### Conformance triage snapshot (2026-02-06)

The current blended pass rate is `47.30%` (`2411/5097`) after baseline stabilization.
The current `oxfmt` pass rate is `21.80%` (`29/133`).
The current `oxfmt` failure split is `parse=12`, `output=92`, `idempotence=0`, and `read=0`.
The current `biome` failure split is `parse=449`, `output=803`, `idempotence=35`, and `read=0`.
The current `prettier` failure split is `parse=1146`, `output=0`, `idempotence=147`, and `read=0`.

`oxfmt` parser queue that should be handled in the parser changestream first.
`js/assignments/issue-16089.js`.
`js/assignments/issue-16704.js`.
`js/comments/assignment-pattern.js`.
`js/comments/import-expression.js`.
`js/import-expressions/grouped.mjs`.
`ts/class/issue-16259.ts`.
`ts/directives/issue-16192.ts`.
`ts/parameters/object-pattern.ts`.
`ts/parenthesis/type-assertion.ts`.
`ts/semicolons/assignment.ts`.
`ts/static-members/non-null-expression.ts`.
`ts/union/issue-16902.ts`.

`oxfmt` formatter queue priority after parser split.
1. `js/comments` attachment and line postfix boundary behavior (`12` cases).
2. `ts/comments` attachment and spacing around unions and method signatures (`7` cases).
3. `ts/assignments` plus `js/assignments` line breaking and semicolon placement (`10` cases).
4. `js/jsx` tree and expression wrapping (`6` cases).
5. `ts/union` leading bar and comment placement policy (`4` formatter cases after parser split).

Style alignment priority queue.
1. Keep compact empty blocks (`{}`) as default across JS, TS, and Destack declaration and block contexts.
2. Keep class, interface, struct, and extension member terminators aligned with JS and TS semicolon style by default.
3. Keep comment attachment behavior stable around member boundaries and semicolon placement with idempotence coverage.
4. Reduce special casing by collapsing equivalent JS and TS versus Destack code paths after each parity fix.

Primary files.
`language/test/fixtures/formatter/transform/`.
`language/test/fixtures/formatter/conformance/`.
`language/test/src/formatter/transform.rs`.
`language/test/src/formatter/conformance.rs`.
`language/formatter/README.md`.
`language/SPECIFICATION.md`.

## 5) Formatter Internals Refinement

Goal.
Reduce risk and improve maintainability without breaking behavior.

Checklist.
- [ ] Reduce complexity concentration in `expression.rs` by factoring coherent helpers.
- [ ] Keep annotation and directive behavior exact while refactoring.
- [ ] Add targeted unit tests near extracted logic.
- [ ] Document hard formatting heuristics and edge cases.

Primary files.
`language/formatter/src/format/expression.rs`.
`language/formatter/src/format/block.rs`.
`language/formatter/src/format/annotation.rs`.
`language/formatter/src/format/directive.rs`.

## 6) Performance Workstream

Goal.
Establish measurable formatter speed and prevent regressions.

Checklist.
- [ ] Add formatter benchmark harness.
- [ ] Add representative benchmark corpora.
- [ ] Establish baseline numbers and publish in local plan updates.
- [ ] Add CI perf guard or at minimum trend reporting.

Primary files.
`language/formatter/Cargo.toml`.
`language/formatter/benches/`.
`language/test/fixtures/formatter/bench/`.
`language/justfile`.

## Immediate Execution Order

1) Fix test infrastructure and establish trustworthy local baseline.
2) Fix CLI format behavior and config wiring.
3) Expand parity fixtures for JS, JSX, and D.TS.
4) Begin formatter internal refactors once test coverage and CLI behavior are stable.
5) Add benchmark harness and measure before and after internal changes.

## Commands for Ongoing Verification

`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter -- roundtrip`.
`cargo test --release -p destack_test --test formatter -- conformance`.
`cargo test --release -p destack_cli --tests fmt`.

## Notes for This Working Session

There are untracked rustc ICE files in repository root from prior runs.
Those files are unrelated to formatter plan work and should remain untouched unless explicitly requested.

## Session Update: Oxfmt Grind (Current)

Completed.
- Fixed empty lambda parameter multiline rendering that emitted a stray trailing comma in broken layouts.
- Standardized static type argument rendering to avoid multiline trailing commas in JS and TS formatting paths.
- Kept statement terminator insertion closer to JS and TS defaults for `import`, `let`, and `using` forms inside block statement lists.
- Updated formatter unit tests and formatter transform fixture expectations for semicolon behavior changes in affected declaration and comment cases.
- Removed `ts/assignments/arrow-function.ts` from `oxfmt-known-failures.txt` after it became a stable pass.

Current verification.
- `cargo test --release -p destack_formatter`: green (`179/179`).
- `cargo test --release -p destack_test --test formatter`: green (`892/892`).
- `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt`: `52/133` passing (`39.10%`), known failures now `81`.

Latest in progress notes.
- Added lambda line prefix deferral wiring from argument wrappers to declaration arrow sites in `language/formatter/src/format/argument.rs` and `language/formatter/src/format/declaration.rs`.
- Removed expression level lambda line prefix pre emission in `language/formatter/src/format/expression.rs` to reduce duplicate or misplaced comment output.
- Kept formatter unit and spec suites green after these refactors.
- Removed `js/comments/arrow.js` from oxfmt known failures after passing conformance.

## Session Update: JS and TS Default Statement Bodies (Current)

Completed.
- Added shared control flow body formatting for wrapped statement blocks in `language/formatter/src/format/expression.rs`.
- Aligned `if`, `while`, `for`, and `for each` body rendering closer to JS and TS defaults for implicit single statement and empty statement bodies.
- Narrowed statement semicolon insertion for `Expression::Statement` control flow cases to avoid double semicolons in `if` chains.
- Added source keyword aware `for each` binding handling and kept fallback behavior minimal for DS only cases.
- Updated formatter transform expectations in:
  - `language/test/fixtures/formatter/transform/statements/control-flow.md`
  - `language/test/fixtures/formatter/transform/declarations/functions.md`
- Preserved green local formatter suites after behavior updates.

Current verification.
- `cargo test --release -p destack_formatter`: green (`179/179`).
- `cargo test --release -p destack_test --test formatter`: green (`892/892`).
- `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt`: `55/133` (`41.35%`), known failures `78`.

Open blockers in this lane.
- Array comment handling now has a dedicated boundary comment path for simple inline element runs, and `js/comments/for-statements.js` is now passing.
- `if` and `else` boundary comment handling was refactored and `js/if/issue-16137.js` is now passing with source style preserved for boundary comments.
- Remaining failures are now concentrated in broader JS and TS comment, JSX, and call chain clusters rather than this specific `if` edge case.

## Session Update: Oxfmt Baseline Cleanup and Targeted JS and TS Alignment (Current)

Completed.
- Removed `js/comments/if.js` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` after it became a stable pass.
- Removed `js/comments/for-statements.js` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` after boundary comment array handling was aligned.
- Removed `js/if/issue-16137.js` from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` after if/else boundary comment handling was aligned.
- Updated `format_if_else_chain` so non block `else` branches are formatted through `format_expression` with explicit directive and annotation handling.
- Updated `format_if_else_chain` spacing rules to avoid `}else` joins when there is no real prefix boundary before `else`.
- Added array boundary comment heuristics in `language/formatter/src/format/expression.rs` to identify concise fill candidates and distinguish boundary comments from internal comments.
- Added source aware if boundary comment handling for both `//` and `/* ... */` cases, including line postfix boundary flushing and safe inline comment preservation when comments sit between then and else.

Current verification.
- `cargo test --release -p destack_formatter`: green (`179/179`).
- `cargo test --release -p destack_test --test formatter`: green (`892/892`).
- `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt`: `55/133` passing (`41.35%`) with `parse=12` and `output=66`.
- Known failures file now loads `78` entries for oxfmt.

Open follow up for immediate next grind.
- Keep JS and TS as the default rule model while preserving DS only divergence only for DS only syntax nodes.
- Move from single case fixes to cluster level refactors, starting with:
  - JS and TS comment attachment in call, chain, and logical contexts
  - JSX comment container and expression child comment placement
  - shared list and boundary comment logic for arrays, argument lists, and call arguments

## Systematic Refactor Strategy

The proper path forward is cluster based refactoring, not single fixture patching.
Cluster selection should follow failure density and shared formatter paths.
Current highest density clusters from oxfmt known failures are:
- `js/comments` (`9`)
- `js/jsx` (`6`)
- `ts/comments` (`6`)
- `ts/union` (`5`)
- `js/calls` (`4`)

For each cluster:
1) Build a representative case pack from oxfmt snapshots and matching local formatter fixtures.
2) Refactor shared formatter logic first, not output strings:
   - annotation attachment policy
   - boundary comment printing
   - list and chain layout behavior
3) Add or update formatter unit and transform fixtures for expected stable behavior.
4) Run full formatter and oxfmt conformance suites, then remove fixed known failures.
5) Repeat with the next highest leverage cluster.

## Session Update: Oxfmt Calls and Chain Layout Pass (Current)

Top rule reaffirmed.
- JS and TS formatting behavior is the default target.
- DS divergence is kept minimal and only for DS only syntax or explicit DS semantics.

Completed.
- Fixed oxfmt `js/calls/react-hooks/issue-16427.js`.
- Fixed oxfmt `js/calls/test.js`.
- Removed both from `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt`.
- Added call argument heuristics in `language/formatter/src/format/expression.rs`:
  - array/object collection tail hugging for hook style `ref, callback, deps` calls
  - test style callback call hugging for long test names (`test`, `it`, `describe`, `only`, `fixme`, etc)
  - tighter non last block callback expansion rules so `useCallback(callback, deps)` remains expanded while `useImperativeHandle(ref, callback, deps)` can stay compact
- Refined chain head promotion and line grouping in `language/formatter/src/format/expression.rs`:
  - merge only targeted member runs before a terminal call (`.property.test.only(...)`)
  - avoid over promoting empty fluent call steps when the chain already starts from a call (`foo().bar().baz(...)`)
  - preserve oxfmt conditional member chain behavior (`internal.getSuspenseCache(client).getFragmentRef(...)`)
- Added a targeted lambda parameter formatting path in `language/formatter/src/format/declaration.rs` for compact `({ ... }, tail)` callback parameter shapes when source is multiline destructuring.
- Updated local formatter fixtures to match stabilized output where needed:
  - `language/test/fixtures/formatter/roundtrip/formatter-0003.ds`
  - `language/test/fixtures/formatter/transform/expressions/ternary.md`

Current verification.
- `cargo test --release -p destack_formatter`: green (`179/179`).
- `cargo test --release -p destack_test --test formatter`: green (`896/896`).
- `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`: green with no regressions.
- oxfmt conformance moved from `69/133` to `70/133` passing (`52.63%`) and known failures moved from `65` to `63`.

Next high leverage output clusters.
1) `js/arguments/*` and `js/call-expression/*` for remaining call list wrapping deltas.
2) `js/jsx/*` for expression child and comment container consistency.
3) `ts/comments/*` and `ts/union/*` for annotation and multiline type layout parity.

## Session Update: Oxfmt Call and Argument Cluster Follow Up (Current)

Top rule reaffirmed.
- JS and TS behavior remains the formatter default.
- DS divergence remains limited to DS only syntax and explicit DS semantics.

Completed.
- Fixed oxfmt `js/arguments/long-curried-call/trailing-comma.js`.
- Fixed oxfmt `js/call-expression/spread-with-callback.js`.
- Updated call argument and chain behavior in `language/formatter/src/format/expression.rs`:
  - added curried call detection for call arguments (`foo(...)(...)`) and forced expansion of long curried call heads
  - kept direct curried call tails grouped in chain line grouping and chain head promotion
  - reduced over eager callback expansion for `callback + trivial tail` forms while preserving block callback expansion for complex and collection tails
  - refined shared chain promotion guardrails so existing member chain behavior remains aligned
- Updated argument prefix annotation handling in `language/formatter/src/format/argument.rs`:
  - filtered blank prefix annotations that sit before separators so separator trivia does not render as argument leading blank lines
  - preserved trailing comma placement before trailing line comments in grouped call/new lists
- Updated formatter transform expectations in `language/test/fixtures/formatter/transform/expressions/complex.md` for stabilized curried call output.
- Updated oxfmt baseline file `language/test/fixtures/formatter/conformance/oxfmt-known-failures.txt` via `--update-known-failures`.

Current verification.
- `cargo test --release -p destack_formatter`: green (`179/179`).
- `cargo test --release -p destack_test --test formatter`: green (`896/896`).
- `cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`: green, no regressions.
- oxfmt conformance moved from `70/133` to `72/133` passing (`54.14%`).
- oxfmt known failures moved from `63` to `61`.

Open blocker in this cluster.
- `js/arguments/empty-lines.js` still has one output delta:
  - one missing preserved blank line before a prefix comment in one call argument case
  - this is annotation placement and boundary trivia handling, not parser failure
  - keep it in known failures for now and fold into the dedicated comments and boundary trivia refactor lane

## Formatter Cleanup Audit: 2026-02-08

### Audit Method

This audit pass focused on formatter source structure and maintainability risk before further conformance grinding.
The audit included module inventory, file and function size profiling, duplication checks, and hotspot inspection for language branching and annotation logic coupling.
The audit also reviewed formatter test harness layout to keep refactors safely verifiable.

### Complexity Snapshot

The table below summarizes current structural concentration in `language/formatter/src/format/`.

| File | Lines | `fn` count | In-file tests |
| --- | ---: | ---: | ---: |
| `language/formatter/src/format/expression/chain.rs` | 3482 | 82 | 0 |
| `language/formatter/src/format/expression/core.rs` | 2987 | 2 | 0 |
| `language/formatter/src/format/annotation.rs` | 2042 | 38 | 17 |
| `language/formatter/src/format/expression/tree.rs` | 1627 | 29 | 0 |
| `language/formatter/src/format/declaration.rs` | 1408 | 16 | 0 |
| `language/formatter/src/format/expression/call.rs` | 1318 | 31 | 0 |
| `language/formatter/src/format/argument.rs` | 1124 | 21 | 8 |
| `language/formatter/src/format/property.rs` | 1009 | 22 | 7 |
| `language/formatter/src/format/expression/tests.rs` | 905 | 71 | 71 |
| `language/formatter/src/format/expression.rs` | 562 | 12 | 0 |

The largest concentration is now in `language/formatter/src/format/expression/chain.rs` and `language/formatter/src/format/expression/core.rs`.
`language/formatter/src/format/expression.rs` is now a thin orchestration module.

### Primary Findings

#### Finding 1: Expression Monolith Risk Is Now Shifted to `chain.rs` and `core.rs`

The original `expression.rs` monolith has been decomposed into focused submodules.
The remaining high-complexity clusters are `language/formatter/src/format/expression/chain.rs` and `language/formatter/src/format/expression/core.rs`.
These files now carry most breakability and emission policy, so they are the primary next cleanup targets.

#### Finding 2: Annotation Deferral and Rendering Are Overcoupled

`language/formatter/src/format/annotation.rs` mixes annotation capture APIs, deferral predicates, source scanning helpers, and rendering policy.
The central `Annotations::format` loop is large and policy-dense (`language/formatter/src/format/annotation.rs:1047` through `language/formatter/src/format/annotation.rs:1487`).
The loop contains cross-cutting rules for ternary, parenthesized, call boundary, parameter separator, lambda arrow, declaration body, and method body behavior.
This design creates implicit dependencies on `expression.rs`, `argument.rs`, `declaration.rs`, and `property.rs`.

#### Finding 3: Signature Duplication Is Mostly Resolved

Shared signature heuristics were centralized into `language/formatter/src/format/signature.rs`.
The remaining work is to keep new signature decisions routed through shared helpers and avoid drift regressions.

#### Finding 4: Source Scanner Duplication Is Mostly Resolved

Shared source scanning helpers were centralized into `language/formatter/src/format/source_scan.rs`.
The remaining work is to remove any new ad hoc scanner growth and keep scanner rules aligned across modules.

#### Finding 5: Property and Member Method Formatting Was Deduplicated

`Property::Method` and `Member::Method` formatting now share method-like helpers in `language/formatter/src/format/property.rs`.
The remaining work is policy simplification and clearer naming in those shared helpers.

#### Finding 6: Expression Test Embedding Is Resolved

Several large formatter modules embed substantial test code directly.
`language/formatter/src/format/expression/tests.rs` now owns the large expression test set, and `language/formatter/src/format/expression.rs` no longer embeds that block.
Other modules still have local tests, but the highest merge conflict hotspot has been removed.

#### Finding 7: JS/TS Baseline Guardrails Need Stronger Encapsulation

Language-specific branching exists in valid syntax-specific areas.
The highest drift risk is in mixed behavior paths where JS/TS default and DS-specific formatting are interleaved.
Type-binary intersection handling in `language/formatter/src/format/expression.rs` is one such hotspot.
The top rule remains required: default to JS/TS behavior, and only diverge for DS-only syntax or the narrow leading `&` style edge.

### Refactor Architecture Target

#### A. Split `expression.rs` into Submodules

Create `language/formatter/src/format/expression/` and keep `language/formatter/src/format/expression/mod.rs` as the orchestrator.
Keep current external function surface stable while moving internals.

Proposed extraction map:
- `language/formatter/src/format/expression/core.rs`: main expression dispatch and precedence wrappers.
- `language/formatter/src/format/expression/call.rs`: call, `new`, and argument list policy.
- `language/formatter/src/format/expression/chain.rs`: chain collection, grouping, and split behavior.
- `language/formatter/src/format/expression/ternary.rs`: ternary collection and output.
- `language/formatter/src/format/expression/type_binary.rs`: type union and intersection flattening and grouping.
- `language/formatter/src/format/expression/tree.rs`: tree and JSX formatting.
- `language/formatter/src/format/expression/analysis.rs`: width and source-length heuristics.
- `language/formatter/src/format/expression/classify.rs`: triviality and breakability classifiers.

#### B. Add Shared Source Scanning Utilities

Create `language/formatter/src/format/source_scan.rs` for shared boundary and whitespace scanners.
Move duplicated `previous_non_whitespace_before_annotation` and `next_non_whitespace_after_annotation` helpers there.
Reuse these helpers from annotation, argument, property, and expression formatters.

#### C. Add Shared Signature Policy Module

Create `language/formatter/src/format/signature.rs` for shared parameter and signature list heuristics.
Move constructor expansion, variadic handling, and single-parameter hugging there.
Reuse in both declaration and property/member formatter paths.

#### D. Split Annotation Policy from Rendering

Keep `language/formatter/src/format/annotation.rs` focused on formatting annotation nodes.
Move deferral predicates to `language/formatter/src/format/annotation_policy.rs`.
Move annotation collection helpers to `language/formatter/src/format/annotation_collect.rs`.
Reduce `Annotations::format` to policy dispatch plus rendering.

#### E. Deduplicate Property and Member Method Output Paths

Introduce shared helpers for method signature and body emission in `language/formatter/src/format/property.rs` or a shared module.
Keep node-specific wrappers minimal.
Ensure one logic path governs dynamic parameter wrapping and method body boundary comment placement.

#### F. Move Large Unit Tests Out of Production Files

Move large formatter tests from production modules into `language/formatter/src/tests/` modules.
Prioritize moving tests from `language/formatter/src/format/expression.rs` first.
Keep smaller local tests in place only where they provide immediate locality value.

### Concrete Cleanup Checklist

#### Phase A: Low-Risk Dedup

- [x] Add `language/formatter/src/format/source_scan.rs` and centralize annotation scanners.
- [x] Replace duplicated scanner functions in:
  - `language/formatter/src/format/annotation.rs`.
  - `language/formatter/src/format/argument.rs`.
  - `language/formatter/src/format/property.rs`.
  - `language/formatter/src/format/expression.rs`.
- [x] Add `language/formatter/src/format/signature.rs` with shared parameter heuristics.
- [x] Replace duplicated parameter/signature helpers in:
  - `language/formatter/src/format/declaration.rs`.
  - `language/formatter/src/format/property.rs`.

#### Phase B: Expression Decomposition

- [x] Introduce `language/formatter/src/format/expression/` submodules (without switching to `expression/mod.rs` yet).
- [x] Move chain logic to `language/formatter/src/format/expression/chain.rs`.
- [x] Move call logic to `language/formatter/src/format/expression/call.rs`.
- [x] Move ternary logic to `language/formatter/src/format/expression/ternary.rs`.
- [x] Move type-binary logic to `language/formatter/src/format/expression/type_binary.rs`.
- [x] Move tree/JSX logic to `language/formatter/src/format/expression/tree.rs`.
- [ ] Move analysis helpers to dedicated files.
- [x] Move classifier helpers to dedicated files.

#### Phase C: Annotation and Method Path Simplification

- [ ] Introduce `language/formatter/src/format/annotation_policy.rs`.
- [ ] Extract deferral predicates from `language/formatter/src/format/annotation.rs`.
- [ ] Shrink `Annotations::format` orchestration complexity.
- [x] Deduplicate `Property::Method` and `Member::Method` formatting in `language/formatter/src/format/property.rs`.

#### Phase D: Test Layout Cleanup

- [x] Move large expression formatter tests out of `language/formatter/src/format/expression.rs`.
- [ ] Keep test names and assertions stable during movement.
- [ ] Verify local unit and fixture suites remain green after relocation.

### Validation Gates for Every Cleanup Step

Every cleanup step must keep these commands green.
`cargo test --release -p destack_formatter`.
`cargo test --release -p destack_test --test formatter`.
`cargo test --release -p destack_test --test formatter-conformance -- --oxfmt --no-parallel`.

Cleanup-only commits should not modify known-failure baselines unless output behavior changes intentionally.

### Prioritized Start Point

Start with Phase A because it yields high maintainability gain with low conformance risk.
Then start Phase B with chain and call extraction because those two clusters currently dominate parity work and regression risk.
Keep JS/TS as default behavior throughout all extraction work, and isolate DS-only divergence to DS-only syntax and the leading `&` style edge.
