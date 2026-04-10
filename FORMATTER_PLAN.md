# Formatter Plan

## Goal

Bring Destack formatter behavior into full alignment with `oxc_formatter` for all supported syntax.
Bring Destack formatter behavior into practical alignment with Prettier to the degree that OXC permits.
Treat unsupported or out of scope syntax through conformance status files, not parser or formatter hacks.

## Rebaseline 2026-04-08

The fixture truth phase is effectively closed.
Shared formatter truth is now aligned to real `oxfmt` under the fixture's actual options.
Local Destack and TS++ truth has been reviewed family by family and is now explicit instead of inferred from current formatter output.
The old `smoke/` fixture lane has been dissolved into `transform/` and `roundtrip/`.
The call-like family is now checkpointed closely enough that it is no longer the active rewrite target.

The current implementation debt is narrower and more trustworthy:

- body-level annotation emission for class, struct, enum, and match surfaces

The binary and logical operator ownership tranche is now checkpointed again.
That checkpoint includes:

- indented flattened tails for logical and mixed binary chains
- binary-local removal of redundant grouping wrappers without reopening the shared parenthesized path
- type-cast comment ownership that stays with the following parenthesized head instead of leaking to the previous statement boundary
- focused unit coverage for logical-chain indentation and redundant binary grouping wrappers
- green fixture lanes for `transform/expressions/binary.md` and `transform/expressions/type-binary.md`

The declarator and assignment fallout tranche is now checkpointed again.
That checkpoint includes:

- statement terminators no longer stealing blank-line-separated comments from the following statement
- assignment shells preserving call-with-type-argument prefix comments and cast-headed logical rhs layout
- variable declaration containers breaking initialized multi-declarator statements one per line without reopening single-declarator indentation
- focused unit coverage for assignment shells and cast-headed logical rhs layout
- green fixture lanes for `transform/expressions/assignment.md` and `transform/declarations/variables.md`

The declaration annotation and type-boundary ownership tranche is now checkpointed again.
That checkpoint includes:

- the stack-overflow path in `transform/declarations/annotations.md`
- parser-side leading-owner preservation for nested leading type unions
- parser recovery for malformed call empty slots now preserving following statement shape again
- signature, mapped-type, conditional-type, class-heritage, and assignment-boundary ownership fixes
- green fixture truth for `transform/declarations/annotations.md`
- focused parser and formatter coverage for nested leading-union ownership and decorated single-member intersections
- a green `cargo test -p destack_parser` baseline

The active next tranche is now body-level annotation emission.
The first files to reopen in that tranche are:

- `format/annotation/sequence.rs`
- `format/declaration/declaration.rs`
- `format/declaration/function.rs`
- `format/expression/primary.rs`
- `format/collection/property.rs`

The expression comment-ownership tranche is now checkpointed again.
That checkpoint includes:

- parser-side separator-boundary ownership for infix type operators
- shared comment and list-boundary ownership fixes
- ternary alternate-boundary ownership that only transfers line comments
- chain head merge parity for short statement-position heads
- focused unit coverage for unary, parenthesized, ternary, and chain boundary cases

The chain and member-call family is now checkpointed again.
That checkpoint includes:

- verified fixture truth for instantiation chains under the fixture's real `line-width` and `indent-width`
- normalized parenthesized ownership for chain routing
- instantiation-root routing into the chain formatter when the instantiation wraps a member chain
- first computed-hop merge parity by blocking the head merge only for true member-intervening comments, not generic computed-gap comments
- unit coverage for head splitting, optional boundaries, blank-line preservation, instantiation chain shape, and short statement-position head merging

The current targeted signals are:

- `comments/comments.md`: green
- `format-ignore-comments`: green
- `transform/expressions/comments.md`: green
- `transform/expressions/comment-boundaries.md`: green
- `comments/attachment.md`: green
- `transform/expressions/calls.md`: no longer blocked on declaration annotation ownership
- `transform/declarations/variables.md`: green
- `transform/declarations/annotations.md`: green

The immediate next targets are:

- `format/annotation/sequence.rs`
- `format/declaration/declaration.rs`
- `format/declaration/function.rs`
- `format/expression/primary.rs`
- `format/collection/property.rs`

## Reopened Strict Audit

The previous formatter audit was too permissive.
It treated family-level similarity and loose analogue mapping as sufficient proof.
That bar was wrong.

The current strict audit standard is:

- exact helper names are expected to resemble real `oxc_formatter` structure
- local wrappers must justify themselves as thin IR adaptation
- any long or heuristic helper with no clear OXC peer is reopened by default
- AGENTS.md compliance is part of the audit, not a follow-up nicety

The following families are explicitly reopened and should not be treated as already signed off:

- `format/operator/binary.rs`
- `format/call/arguments.rs`
- `format/annotation/sequence.rs`
- `format/declaration/assignment.rs`
- `format/declaration/sequence.rs`
- `format/declaration/statement.rs`
- `format/declaration/type.rs`
- `format/expression/declarator.rs`
- `format/expression/object.rs`
- `format/expression/parentheses.rs`
- `format/expression/primary.rs`
- `format/operator/assign.rs`
- `format/operator/type.rs`
- `format/operator/types.rs`
- `format/operator/union.rs`

The following helpers are explicitly reopened as suspicious examples:

- `binary_operator_expression_precedence`
- `write_default_flattened_non_head_operand`
- `prefix_annotations_with_raw_prefix_comments`
- `prefix_annotations_without_decorators`
- `decorator_prefix_annotations`
- `call_has_boundary_comments`
- `write_call_argument_list`
- `receiver_is_await_wrapped`
- `chain_overflows_in_type_binary_left`
- `write_chain_base`
- `write_chain_operation`
- `write_expanded_chain`
- `chain_has_parent_intervening_break_or_comment`
- `expression_is_poorly_breakable_member_or_call_chain`
- `expression_is_short`
- `argument_is_short`
- `format_program_statement_sequence`
- `format_block_statement_sequence_for_block`
- `format_block_statement_sequence`
- `format_block_body_wide`
- `format_block_body_narrow`
- `empty_block_requires_expanded_layout`
- `format_anonymous_class_heritage`
- `format_struct_or_class_declaration`
- `declarator_drops_parenthesized_value_wrapper`
- `declarator_layout`
- `format_declarator`
- `struct_literal_layout`
- `parenthesized_is_in_assignment_value_context`
- `format_preserved_parenthesized_expression`
- `format_primary_array_expression`
- `assignment_expression_layout`
- `format_binary_operand_with_grouping_parentheses`
- `write_expression_with_inline_prefix_annotations`
- `format_type_binary_expression`
- `should_drop_type_binary_left_parentheses`
- `format_type_union_binary_layout`

## Compensation Inventory

The list below is the current concrete inventory of formatter complexity that looks compensatory rather than structural.
Each item names the local hotspot and the source-level change that should eventually delete it.

### 1. Cast and assertion seam recovery

- `format/context/comment.rs`: `get_type_cast_comment_index` and `comment_is_type_cast` infer cast wrappers from raw comment placement and source text instead of consuming parser-owned structure.
- `format/expression/parentheses.rs`: `is_type_cast_comment_node`, `format_type_cast_comment_node`, and `type_cast_comment_nodes` reconstruct closure or JSDoc cast shells from comment state, printed cursor state, and parenthesis scanning.
- `format/expression/shape.rs`: `expression_type_cast_comment_head_start` walks the left spine and reuses cast-comment inference as a layout signal.
- `format/declaration/semicolon.rs`: semicolon handling still branches on cast-comment classification instead of relying on structural ownership.
- `format/operator/types.rs`, `format/operator/type.rs`, and `parse/expression/argument.rs`: expression-side `as`, `satisfies`, angle assertions, and `as const` are not modeled as one coherent syntax family, so the formatter keeps re-synchronizing them manually.
- Root fix direction: make cast-family seams explicit in CST or AST and source spans, including separator ownership and comment ownership, then delete cast-comment recovery from the formatter.

### 2. Type payload boundary reconstruction

- `format/operator/type.rs`: `type_expression_boundary_start` reconstructs comment ownership from parent node kind and token archaeology for `:`, `?:`, `=>`, mapped values, remaps, and similar payload seams.
- `format/context/source.rs`: `raw_type_position_comments_for`, `raw_comments_in_separator_for`, and `raw_boundary_comments_in_range` compensate for missing uniform boundary ownership by filtering comment slices after the fact.
- `format/declaration/assignment.rs` and `format/chain/argument.rs`: assignment and argument formatting depend on `type_expression_boundary_start` and `raw_type_position_comments_for`, so they inherit the same compensation model.
- `format/expression/mapped.rs`: mapped type clause printing still has local separator comment handling around `as` remaps and value seams.
- `parse/type.rs` and `parse/expression/continuation.rs`: separator spans now exist for some payload seams, but the model is still partial and not yet the single source of truth for every explicit type payload boundary.
- Root fix direction: define one parser or source ownership rule for type payload boundaries and make formatter consumers rely only on `NodeSpanType` plus normal attached comment queries.

### 3. Leading separator wrapper ownership for unions and intersections

- `format/operator/union.rs`: the root shell, first separator shell, operand separator shell, and head comment ownership are all split across many helpers because leading `|` and `&` ownership is still underspecified.
- `format/operator/union.rs`: helpers like `type_union_root_leading_comment_groups`, `type_union_root_comments_after_leading_separator`, `type_binary_operand_separator_span`, and the surrounding split or promote logic are all symptoms of missing generic separator or head ownership.
- `parse/expression/expression.rs` and `parse/type.rs`: leading separator spans are now partially preserved, but the formatter still needs union-specific assembly logic on top of them.
- Root fix direction: make root head ownership and operand separator ownership generic in parser or source, then shrink `union.rs` down to layout rather than ownership recovery.

### 4. Parenthesized wrapper meaning leaks

- `format/expression/parentheses.rs`: parenthesized formatting still decides whether a wrapper is plain grouping, a cast shell, an assignment shell, a decorator shell, a tree shell, or a newline-preserving shell.
- `format/declaration/type.rs`: `parenthesized_wraps_decorated_class_extends_head` and `parenthesized_wraps_prefix_annotated_class_extends_head` are formatter-side recovery for declaration-ish expression wrappers in heritage position.
- `format/operator/type.rs`: parenthesized type handling depends on those same heritage wrapper helpers.
- Root fix direction: carry wrapper meaning in parser or source ownership where the syntax actually creates the shell, and keep plain parenthesized nodes semantically boring.

### 5. Decorator and statement-like annotation classification

- `format/annotation/sequence.rs`: `prefix_annotations_without_decorators` and `decorator_prefix_annotations` split annotation rendering with local policy logic that depends on inferred owner shape.
- `format/collection/property.rs` and `format/collection/member.rs`: class and struct member formatting has to explicitly route through the decorator-only prefix path to preserve statement-like decorator layout.
- `format/declaration/type.rs`, `format/expression/member.rs`, and `format/expression/parentheses.rs`: decorated class expressions in `extends`, member access, and grouped positions still rely on formatter-side detection of declaration-ish expression forms.
- `parse/expression/expression.rs` and `parse/property.rs`: decorator attachment is structurally better than before, but the formatter still lacks one explicit notion of statement-like decorated owner versus inline decorated owner.
- Root fix direction: make decorator owner classification explicit enough in parser or source so formatter policy only decides vertical versus inline layout at real syntax boundaries.

### 6. Raw comment cursor and speculative inspection

- `format/context/comment.rs`: several comment queries still depend on the live printed cursor and on ad hoc visibility limits.
- `format/expression/parentheses.rs` and `format/expression/shape.rs`: cast detection currently mixes raw source scanning with printed comment state, which makes speculative layout harder to reason about.
- Root fix direction: keep ownership queries purely structural and isolate speculative printing from the live raw comment cursor.

### 7. Remaining mixed-family operator buckets

- `ast/tree/expression.rs`: `Expression::TypeBinary` currently mixes expression-side `as` and `satisfies` with unrelated operators like `in`, `is`, `extends`, and `implements`.
- `ast/tree/operator.rs`: `TypeBinaryOperator` and `TypeUnaryOperator::AsConst` split one visible surface family across multiple operator buckets.
- `parse/expression/operator.rs` and `parse/expression/argument.rs`: parsing rules already special-case the cast-family forms, which is evidence that the shared node shape is carrying multiple incompatible invariants.
- Root fix direction: either split the syntax family in CST or AST or carry equivalent first-class metadata so formatter and source code stop redispatching one generic bucket into several semantic subfamilies.

### 8. Tranches that still need the same census treatment

- `format/call/arguments.rs`, `format/chain/expression.rs`, `format/declaration/sequence.rs`, and `format/declaration/statement.rs` are already reopened in the strict audit, but they still need the same root-cause inventory pass rather than only helper-level suspicion.
- Root fix direction: keep extending this section as each reopened tranche gets reviewed, and only mark a tranche re-cleared after the source of each local complexity spike is either justified or removed.

### 9. Secondary boundary-heavy surfaces from the second sweep

The files below use a lot of direct token and comment range probing.
Not every query here is necessarily wrong, but they are the next places to inspect for the same ownership-model gap.

- `format/declaration/dependency.rs`: alias and `from` clause boundaries still recover separator and gap comments by token archaeology.
- `format/expression/ternary.rs`, `format/expression/conditional.rs`, `format/expression/control.rs`, and `format/expression/statement.rs`: `?`, `:`, `else`, and body-boundary comments are still assembled from explicit token spans and raw range queries.
- `format/call/list.rs`, `format/collection/list.rs`, `format/tree/child.rs`, and `format/tree/argument.rs`: generic list and child ownership still relies on direct gap scans between delimiters and elements.
- `format/operator/assign.rs` and `format/expression/declarator.rs`: assignment and declarator boundary comments still reconstruct ownership from token gaps.
- `format/chain/expression.rs` and `format/chain/member.rs`: chain and member merges still inspect raw source and comment gaps directly.
- `format/expression/object.rs` and `format/collection/property.rs`: object and property tail boundaries still gather comments by scanning around braces and field spans.

The current inventory should now be treated as complete for the major root-cause families and complete enough to plan the target CST or AST model.
It is still not a proof that every single range query above is compensatory.
Those files remain audit-open until each query is either justified as ordinary delimiter handling or replaced by structural ownership.

## Target Syntax Model

The current formatter should move toward one explicit syntax model instead of growing more ownership recovery.
The intended target is:

- CST or AST carries syntax distinctions that change comment ownership, separator ownership, wrapper meaning, or parenthesis rules
- source spans expose those distinctions through `Head`, `Leading`, and `Separator` ownership
- formatter renders only structural ownership and layout, not inferred intent
- DIR or IR may merge syntax families later when their semantic meaning is truly shared

### Cast and assertion family

Expression-side `as`, `satisfies`, and angle assertions should stop being mixed into the generic type-binary bucket.
The upstream AST uses distinct expression variants for those forms, and the parser emits them directly.
The target here should match that structure closely instead of inventing a local generic assertion wrapper.

Cross-checked upstream shape in `~/symbol/oxc`:

- `Expression::TSAsExpression`
- `Expression::TSSatisfiesExpression`
- `Expression::TSTypeAssertion`

Destack target CST or AST shape:

- `Expression::As { expression, ty }`
- `Expression::Satisfies { expression, ty }`
- `Expression::TypeAssertion { ty, expression }`

`as const` should not be a separate expression variant.
Upstream handles it as an `as` expression whose type payload is a `const` type reference.
The Destack target should follow that model and remove `TypeUnaryOperator::AsConst` as a separate surface encoding.

The generic operator buckets should then narrow:

- `TypeBinaryOperator` should stop carrying expression-side `Cast` and `Satisfies`
- `TypeUnaryOperator` should stop carrying expression-side `AsConst`
- true type-relation operators can stay in those enums

DIR or IR target:

- no planned DIR changes
- no planned compiler IR changes
- syntax ownership, separator ownership, and parenthesis behavior should be fixed in CST or AST, parser, and source mapping, not pushed downstream

### Payload seams

Payload seams should be generic and structural.
`Separator` spans should be set consistently for payload children after:

- `as`
- `satisfies`
- angle assertions
- `:`
- `?:`
- `=>`
- mapped `as` remaps
- mapped `:` values
- conditional `?` and `:`
- union and intersection separators after the first operand

The formatter should then ask one generic question about separator-owned comments rather than rebuilding one ownership rule per syntax family.

### Source part model

The source-map layer should become the single ownership model for formatter comment queries.
This part is a local extension rather than an upstream AST concept, but it is the right place to encode the ownership that the formatter currently reconstructs.

The intended source-part set is:

- `Head`: the first semantic body token of the node
- `Leading`: prefix-owned syntax before `Head`
- `Separator`: the infix or delimiter seam before a payload child
- `Trailing`: the tail seam after the last payload child and before the closer or terminator
- `Segment(u16)`: optional extra seams for multi-part nodes that need more than one named separator

The formatter should rely on structural attachment to these source parts instead of raw range scans.

The final comment model should have one ownership system only:

- every comment attaches to a `SourcePartKey`
- `attached_to` is legacy transition state and should be removed
- `CommentPosition` is redundant transition state and should be removed
- no token-relative fallback field should survive in the final model

### Parenthesized wrappers

Plain parenthesized nodes should mean grouping only.
If a wrapper has special ownership meaning, that meaning should be represented structurally in CST or AST or through source spans at parse time.
The formatter should not infer cast wrappers, decorator wrappers, or boundary-owning wrappers from comment state and raw source scans.

### Decorator owners

Decorator attachment should preserve whether the owner is statement-like or inline.
That classification should be explicit enough that the formatter only chooses layout policy at real syntax boundaries instead of rediscovering declaration-like expressions in heritage, member, or grouped positions.

### Rejected local expansion

The broad wrapper-aware `expression_is_type_position` expansion is explicitly rejected.
It widened formatter-side inference across `Property`, `Member`, `Argument`, `Parameter`, `Declaration`, and `Declarator` wrappers instead of adding structural ownership.
If any of those wrappers need to propagate type position, the propagation must come from parser, AST, or source ownership, not from a broader formatter walk.

## Post-Cut Cross Audit

The formatter now has the cast-comment heuristic path removed.
That removal intentionally reopens a small set of tests, and it also makes the remaining ownership debt easier to see.

The current post-cut inventory, cross-checked against `FORMATTER_MAP.md`, is:

- `format/operator/type.rs`, `format/context/source.rs`, `format/declaration/assignment.rs`, `format/chain/argument.rs`, and `format/expression/mapped.rs`: type payload boundary ownership is still reconstructed through boundary-start helpers and separator comment scans.
- `format/expression/mapped.rs`: the remap block-comment splice and value-to-`}` trailing comment scan were removed on purpose, so any new failures here now indicate missing parser or source ownership instead of printer recovery.
- `format/operator/union.rs`: union and intersection ownership is still too custom around root leading separators, first operand separator comments, and promoted head comments.
- `format/expression/parentheses.rs`, `format/expression/member.rs`, `format/declaration/type.rs`, and `format/context/parenthesized.rs`: parenthesized wrapper meaning still leaks through formatter-side view logic and declaration-ish wrapper checks.
- `format/annotation/sequence.rs`, `format/collection/property.rs`, `format/collection/member.rs`, and `format/declaration/type.rs`: decorator layout still depends on formatter-side owner classification and split annotation writers.
- `format/declaration/signature.rs`, `format/declaration/function.rs`, `format/declaration/declaration.rs`, `format/declaration/statement.rs`, `format/expression/ternary.rs`, `format/expression/conditional.rs`, and `format/expression/control.rs`: boundary comments for `:`, `?`, `=>`, `else`, braces, and similar control seams still use direct token-gap ownership instead of one generic parser or source boundary model.
- `format/declaration/dependency.rs`, `format/call/list.rs`, `format/collection/list.rs`, `format/tree/child.rs`, and `format/tree/argument.rs`: list and separator ownership is still assembled by local gap scans and should be reviewed against one shared separator ownership model.
- `format/chain/expression.rs`, `format/chain/member.rs`, and `format/call/grouped.rs`: chain and grouped-call behavior still uses direct source and comment-gap inspection in places where map parity is only indirect.

The map also confirms one important architectural point:
the old cast-comment path had an upstream utility peer in `src/utils/typecast.rs`, but the surrounding local ownership model is still different enough that a straight formatter port is not acceptable.
The replacement still needs to be structural in parser, AST, or source ownership rather than a revived formatter utility.

## Verified Family Checklist

Only tranches listed here as re-cleared may be treated as signed off.
Any red in the listed verification commands reopens that tranche immediately.

- `Tranche 0. Scope Freeze And Audit Hygiene`: open.
The checklist exists, but the hostile diff audit is still in progress.

- `Tranche 1. Call-Like Expression Parity`: re-cleared on 2026-04-10.
Verification: `cargo test -p destack_formatter`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'calls.md'`, and `cargo test -p destack_formatter inline_snapshot_matcher_call_width_behavior`.
Latest strict-audit fix: removed the static-member-with-following-suffix call branch in `format/call/expression.rs` and tightened `import.meta.resolve` detection in `format/call/pattern.rs`.

- `Tranche 2. Comment Attachment And Chain Boundary Parity`: re-cleared on the currently tracked signal lanes.
Verification: `cargo test -p destack_test --test formatter -- 'comments/comments.md'`, `cargo test -p destack_test --test formatter -- 'expressions/comments.md'`, `cargo test -p destack_test --test formatter -- 'expressions/comment-boundaries.md'`, `cargo test -p destack_test --test formatter -- 'comments/attachment.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'calls.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'chains.md'`, and `cargo test -p destack_formatter`.
Strict source audit of the touched chain internals is still open even though the current signal lanes are green.

- `Tranche 3. Chain Family Parity`: re-cleared on the currently tracked signal lanes.
Verification: `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'chains.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'calls.md'`, and `cargo test -p destack_formatter`.
Strict source audit of `format/chain/expression.rs`, `format/chain/member.rs`, and related helpers is still open.

- `Tranche 4. Declarator And Assignment-Like Parity`: re-cleared on 2026-04-10.
Verification: `cargo test -p destack_test --test formatter -- 'assignment.md'`, `cargo test -p destack_test --test formatter -- 'variables.md'`, and `cargo test -p destack_formatter`.

- `Tranche 5. Type, Binary, And Union Parity`: re-cleared on 2026-04-10.
Verification: `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'binary.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'type-binary.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'annotations.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'classes.md'`, and `cargo test -p destack_formatter`.
This tranche stays audit-open until the hostile diff review is finished across `format/operator/binary.rs`, `format/operator/type.rs`, `format/operator/types.rs`, `format/operator/union.rs`, and the touched parser or context support.
Latest strict-audit fix: the flattened binary rhs owner now groups only against the same binary kind instead of any adjacent binary expression.

- `Tranche 6. Collection, Property, And Module Ownership`: re-cleared on 2026-04-10 for the currently verified property and object lanes.
Verification: `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'objects.md'`, `CARGO_INCREMENTAL=0 cargo test -p destack_test --test formatter -- 'classes.md'`, and `cargo test -p destack_formatter`.
This tranche stays audit-open until the hostile diff review is finished across `format/collection/property.rs`, `format/collection/member.rs`, `format/expression/object.rs`, and any touched quote-property support.
Local policy: statement-like decorators stay on their own line above class or struct members, while parameter decorators may stay inline.

- `Tranche 7. Statement, Control, And Annotation Ownership`: open.
Body-level annotation emission remains the active unfixed family in this tranche.
Current probes: `cargo test -p destack_formatter test_format_match_expression_cases -- --nocapture` and `cargo test -p destack_formatter test_format_switch_expression_cases -- --nocapture`.

- `Tranche 8. Tree Discipline Pass`: open.
This tranche has not been re-audited in the current strict pass.

## Why This Audit Comes First

The current formatter still contains logic from an older generation of work, from before exact OXC parity was the standard.
That older logic makes conformance work less trustworthy because point fixes can stack on top of local policy that should not exist in the first place.
The purpose of the audit is to separate:

- real OXC analogues
- local support layers for AST, trivia, and ownership
- local-only formatting policy
- code that should be structurally rewritten to match OXC

Only after that separation is explicit does it make sense to drive the conformance burn-down family by family.

## Canonical Sources

The canonical structural inventory is [FORMATTER_MAP.md](/Users/florian/symbol/destack-6/FORMATTER_MAP.md).
The map is currently maintained manually and cross-audited directly against the current `oxc_formatter` source tree in `~/symbol/oxc`.
The local fixture-truth audit helper lives at [audit_formatter_fixtures_against_reference.py](/Users/florian/symbol/destack-6/language/test/src/formatter/scripts/audit_formatter_fixtures_against_reference.py), but it is temporary scaffolding rather than a canonical source.
The map is no longer a coarse family summary.
It is the reviewed item-level census for the local formatter surface and the current `oxc_formatter` source tree.

## Audit Status

The reviewed item-level pass is still in progress.

The old copied counts in this plan drifted and should not be trusted.
The live structural inventory is [FORMATTER_MAP.md](/Users/florian/symbol/destack-6/FORMATTER_MAP.md).
Use the reopened family list below and the map's current file sections as the source of truth.

Current high-risk families remain:

- `format/expression/declarator.rs`
- `format/operator/binary.rs`
- `format/operator/assign.rs`
- `format/operator/union.rs`
- `format/annotation/sequence.rs`
- `format/declaration/sequence.rs`
- `format/declaration/statement.rs`

Current root blockers that must be fixed structurally, not compensated around:

- speculative formatter inspection still mutates the live raw comment cursor in some families
- trailing raw comment ownership is not yet modeled through one OXC-shaped query
- `format/call/arguments.rs` still carries local grouped-argument predicate drift around short or simple checks
- comment attachment truth is now explicit, but several attachment owners still do not follow it
- binary and logical operator ownership still depends on local flattening and break heuristics that do not yet fully mirror OXC

## Ground Rules

- `oxfmt` and `prettier` are probes, not task lists.
- The unit of parity work is a formatter family, not an individual failing fixture.
- The unit of audit is an item, not just a file or family.
- Unsupported surface belongs in `status.json`, not in formatter logic.
- If `oxfmt` and `prettier` disagree, prefer OXC unless the syntax is unsupported or impossible in Destack.
- Structural cleanup should come before output-level polishing.
- Family-level resemblance is not enough: the local decomposition should also be OXC-shaped.
- Long, deeply nested, or vaguely named catch-all helpers are suspect until the audit proves otherwise.
- Point fixes are only acceptable near the end, once family-level divergence is small and the family already has an OXC-shaped decomposition.

## Active Conformance Surface

The active parity target is supported JS, JSX, and TS surface.
Everything else should stay out of the burn-down unless we explicitly widen scope.

In scope:

- `language/test/fixtures/conformance/formatter/oxfmt/tests/js/**`
- `language/test/fixtures/conformance/formatter/oxfmt/tests/ts/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/js/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/jsx/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/typescript/**`

Out of scope by default:

- `language/test/fixtures/conformance/formatter/prettier/tests/angular/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/css/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/flow/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/flow-repo/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/graphql/**`
- `language/test/fixtures/conformance/formatter/prettier/tests/handlebars/**`
- other non JS, non JSX, and non TS Prettier suites

## Last Recorded Conformance Baseline

The local formatter crate is green.

- `cargo fmt -p destack_formatter`
- `cargo check -p destack_formatter`
- `cargo test -p destack_formatter -- --format=terse`

The last recorded suite backlog in this plan is:

### Oxfmt

- total cases: `214`
- passed: `90`
- failed: `124`
- pass rate: `42.06%`
- failure kinds: `parse=1`, `output=121`, `idempotence=2`

### Prettier

- total cases: `3230`
- ignored from status: `1695`
- known failures loaded: `754`
- unexpected regressions: `192`
- known failures now passing: `30`

These numbers should be refreshed after the next major family rewrite checkpoint.

## What The Audit Already Resolved

The audit did not just classify code.
It also cleaned up several real problems that were blocking further parity work.

Completed:

- parser and trivia ownership around parenthesized leading comments was pushed upstream
- generic owned-comment machinery was removed from the formatter context path
- several old formatter-only helper families were deleted or narrowed
- several old suspicious helper names and wording patterns were scrubbed from the formatter surface
- a number of obvious family-level mismatches were already burned down in union, signature, statement, and type paths
- the map is now regenerated instead of hand-maintained

Important correction:

- some earlier cleanup wins are real and should stay
- some earlier confidence was too optimistic
- the call and chain families are still structurally suspect
- the remaining work must be driven by the reviewed item census, not by vibes

Recent structural progress:

- `format/call/layout.rs` is gone
- `CallArgumentSignals` is gone
- `scan_call_argument_list` is gone
- `format_decided_call_argument_list` is gone
- `format_call_arguments_with_group` is gone
- the call family now routes through direct grouped-layout selection in `format/call/arguments.rs`
- the call family again has an explicit broken-out argument path for blank-line preservation instead of silently collapsing those separators
- the compensatory call separator-comment helpers were removed again because they were not OXC-shaped
- the chain family now uses explicit tail-group terminology and a `TailChainGroups` builder/container instead of passing raw tail-line vectors through the live path
- `format/chain/expression.rs` now has explicit one-line and expanded writers instead of keeping all rendering orchestration in one block
- the declarator path no longer carries the closure-cast rhs carveout or the nested-call and block-static-argument predicates
- the shared assignment-like chain query now follows OXC short-argument and complex-type-argument structure more closely
- the parenthesized type path now uses one OXC-shaped parentheses-necessity split instead of the old `parenthesized_type_can_drop_*` lattice
- `format/operator/assign.rs` no longer carries the index-operand or keyword-prefixed-chain carveouts
- `format/operator/assign.rs` now has a separate `AssignmentExpressionLayout` plus layout-selection helper instead of a write-or-return monolith
- `format/expression/control.rs` is clean of in-function absolute annotation references in the hot adjacent-argument and if-else paths
- `format/operator/binary.rs` no longer carries the clean short-circuit layout, trailing coalesce layout, or parenthesized binary wrapper-drop helpers
- `format/operator/binary.rs` now routes ordinary binaries through the shared flattened layout, with only the still-needed parenthesized logical support left in place
- FIR line-suffix naming now matches upstream and the local `reserved_width` extension is gone
- formatter comment access now uses upstream-shaped `Comments` and `SourceText` surfaces instead of the old raw `&str` cursor shape
- the call family now has the upstream long-curried-call branch, though integration-level confirmation is still blocked by a nightly compiler ICE outside the formatter crate
- the call-to-chain handoff is now narrower and owned by the call family instead of the generic operator dispatcher
- call-only argument-layout special cases no longer leak into `new` expression formatting
- `format/expression/member.rs` no longer carries the extra declarator-pattern and assignment-left nesting checks in static-member layout

Current call-family reopen notes:

- blank lines between arguments were briefly dropped in the local port and have now been restored through the explicit broken-out path
- the family still lacks some upstream special cases and should not be considered closed
- the long-curried-call branch is now present
- the multiline-template-only branch is now present
- the remaining open questions are the call-like special cases around import, test, and module-import paths, and whether any of them should be intentionally excluded as non-syntax policy

## Completion Criteria

This work is done when all of the following are true:

- the formatter uses an OXC-style raw comment model rather than parser-side semantic comment ownership
- ordinary comments and documentation comments both follow the same raw comment pipeline
- the ordinary JS and TS formatter families are structurally aligned with OXC
- the remaining local-only policy in ordinary JS and TS families is close to zero
- unsupported syntax is cleanly classified in status files instead of leaking into formatter logic
- conformance results move toward full OXC parity and toward the highest Prettier correctness that OXC-compatible behavior allows

Tree syntax is a separate case.
There is no literal OXC printer to copy there.
For tree-specific code, the target is OXC-style discipline rather than literal OXC parity.

## Current Family Verdicts

### Highest Risk

- argument and call list shaping
- member chain shaping
- assignment, declarator, and control interaction
- type, binary, and union shells

### Medium Risk

- collection and property ownership
- import and export shell ownership
- documentation and annotation ownership

### Separate Discipline Pass

- tree literal, child, attribute, and argument layout

## Trivia Model Target

The target trivia model is the strict OXC shape.
That means:

- raw comments only in the tree
- no semantic AST owner on parser trivia
- no blank trivia system
- formatter-side cursor and query state for comments
- formatter families decide leading, trailing, and dangling comment roles lazily

Target nouns:

- keep one raw `Comment` model
- remove `CommentTrivia`
- remove `Blank`
- remove `BlankTrivia`
- remove `TriviaRef`
- remove `TriviaBoundary`
- remove parser-side semantic comment ownership

Target formatter structure:

- add an OXC-style formatter `Comments` subsystem
- keep a printed cursor and snapshot or restore support for speculative formatting
- expose comment queries like `comments_before`, `comments_after`, `comments_in_range`, and trailing-comment resolution
- move comment role decisions into formatter families instead of parser trivia

Strict OXC note on docs:

- documentation comments should also become raw comments
- the old semantic `Doc` annotation system should be removed repo-wide as part of this cutover
- decorators remain semantic AST
- comments and docs should stop using separate pipelines

## Execution Sequence

This is the concrete sequence for closing out the current formatter diff.
The first coherent tranche is still call-like expressions, but that tranche is now checkpoint-ready rather than the active rewrite target.
The next active tranche is comment attachment and chain or binary boundary ownership.

### Tranche 0. Scope Freeze And Audit Hygiene

This tranche keeps the work focused on structural parity instead of fresh point fixes.

Checklist:

- [ ] keep `FORMATTER_MAP.md` regenerated after every family checkpoint
- [ ] keep `FORMATTER_PLAN.md` updated with the current family verdicts and next open tranche
- [ ] keep new unsupported surface out of formatter logic and in status files instead
- [ ] keep AGENTS.md cleanup inside each family rewrite rather than as a final sweep

### Tranche 1. Call-Like Expression Parity

This is the first active parity tranche because it is self-contained enough to finish and it feeds both chain routing and declarator shape.
The scope is the full call-like seam rather than only the local `call/` folder.

Primary local files:

- `language/formatter/src/format/call/mod.rs`
- `language/formatter/src/format/call/expression.rs`
- `language/formatter/src/format/call/arguments.rs`
- `language/formatter/src/format/call/argument.rs`
- `language/formatter/src/format/call/pattern.rs`
- `language/formatter/src/format/operator/expression.rs`
- `language/formatter/src/format/chain/simple_argument.rs`
- `language/formatter/src/tests/call/mod.rs`
- `language/formatter/src/tests/call/argument.rs`
- `language/formatter/src/tests/call/comment.rs`
- `language/formatter/src/tests/call/layout.rs`

Target local file split:

- `call/mod.rs`: export surface only
- `call/expression.rs`: call and instantiation entrypoints, callee handling, and trailing-comment ownership
- `call/arguments.rs`: top-level argument layout selection and list rendering
- `call/argument.rs`: one argument writer and local AST adaptation
- `call/pattern.rs`: call-only special-case probes and call-pattern helpers

Checklist:

- [x] split the current `call/arguments.rs` by owner instead of keeping one giant mixed file
- [x] make the top-level routing order match OXC `call_like_expression`
- [x] keep call-only special cases restricted to actual calls and not `new` or unrelated shells
- [x] make the call-to-chain routing seam explicit and narrow
- [x] keep shared simple-argument ownership in one place for the whole call-like seam
- [x] port grouped-first and grouped-last layout logic helper by helper against OXC
- [x] port or recreate the relevant OXC unit coverage for grouped layout, curried calls, import calls, test calls, and hook dependency arrays
- [x] make the local call unit tests green after the split
- [x] make targeted formatter integration cases for calls, `new`, import calls, and optional-call comment boundaries green

Progress notes:

- 2026-04-08: narrowed chain routing in `operator/expression.rs` so standalone `Member`, `Index`, `Instantiation`, `Maybe`, and `Must` nodes no longer route into member-chain formatting just because they contain an inner call-like expression
- 2026-04-10: removed the non-OXC static-member-with-following-suffix branch from `call/expression.rs` after the hostile audit and restored the upstream fallback path, which fixed the `toMatchInlineSnapshot` width regression without fixture compensation
- 2026-04-08: re-verified `template-literal-argument` and inline hop comments directly against the local OXC formatter, then updated the stale hop-comment unit expectation instead of reverting the source fix
- 2026-04-08: split the old mixed `call/arguments.rs` into `call/argument.rs`, `call/arguments.rs`, `call/grouped.rs`, and `call/list.rs`, then re-verified `cargo test -p destack_formatter tests::call:: -- --format=terse`
- 2026-04-08: re-ran `cargo test -p destack_test --test formatter transform/expressions/calls.md` after the split and confirmed the call-like slice remains green, with only the three known binary or declarator failures left in that fixture
- 2026-04-08: re-ran `cargo test -p destack_test --test formatter comment-before-method-call` to confirm the static-member and hop-comment seam remains stable after the split
- 2026-04-08: aligned `call/pattern.rs` against the upstream special-case probes for two-argument test calls, negative non-literal test names, Angular-style setup wrappers, three-argument hook layouts, and `import.meta.resolve`, then added the corresponding local call-family unit coverage
- 2026-04-08: re-verified `cargo test -p destack_formatter tests::call:: -- --format=terse` with 31 passing tests and re-ran `cargo test -p destack_test --test formatter transform/expressions/calls.md`, which still reports only the same three binary or declarator failures

Primary OXC references:

- `~/symbol/oxc/crates/oxc_formatter/src/print/call_like_expression/mod.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/print/call_like_expression/arguments.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/call_expression.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/simple_argument.rs`

Close criteria:

- `call/arguments.rs` is reduced to layout selection and list writers
- per-argument logic no longer lives in the main list file
- call-to-chain routing is stable and no longer mixed into unrelated owners
- the family has no remaining helper whose ownership is unclear against OXC
- local call tests and targeted formatter integration cases pass

Current tranche boundary:

- structurally, the `call/` family split is now stable enough to keep
- behaviorally, the `transform/expressions/calls.md` fixture no longer reports call-like failures
- the remaining failures inside that fixture belong to binary or declarator formatting, not to the call-like tranche
- the direct-list probes and grouped-layout owners have been audited against the upstream call-like implementation closely enough to treat this family as a checkpoint-ready slice
- further work in this area should only come from later chain or declarator fallout, not from unresolved call-family ownership

### Tranche 2. Comment Attachment And Chain Boundary Parity

This tranche is now the active implementation target.
The call-like family is stable enough that the remaining failures cluster around comment attachment, chain-member gaps, and binary-comment boundaries rather than around raw call-family ownership.

Primary local files:

- `language/formatter/src/format/context/comment.rs`
- `language/formatter/src/format/expression/member.rs`
- `language/formatter/src/format/chain/mod.rs`
- `language/formatter/src/format/chain/expression.rs`
- `language/formatter/src/format/chain/groups.rs`
- `language/formatter/src/format/chain/member.rs`
- `language/formatter/src/format/chain/simple_argument.rs`
- `language/formatter/src/format/operator/binary.rs`

Primary fixtures and tests:

- `language/test/fixtures/formatter/transform/comments/attachment.md`
- `language/test/fixtures/formatter/transform/expressions/comments.md`
- `language/formatter/src/tests/call/comment.rs`

Checklist:

- [ ] make `comments/attachment.md` green without adding new attachment-specific heuristics
- [ ] make the remaining non-directive failures in `transform/expressions/comments.md` green
- [ ] narrow comment attachment ownership in `context/comment.rs` toward one OXC-shaped query model
- [ ] make `chain/expression.rs` orchestration-only where possible
- [ ] move extraction and normalized chain-part building into `chain/member.rs`
- [ ] keep `chain/groups.rs` focused on OXC-like head and tail grouping only
- [ ] delete local grouping heuristics that do not map to `member_chain`
- [ ] port or recreate the relevant OXC tests for head or tail splitting, optional calls, computed access, comment-preserving member hops, and binary-comment boundaries
- [ ] make chain-specific formatter unit tests and targeted integration fixtures pass

Primary OXC references:

- `~/symbol/oxc/crates/oxc_formatter/src/formatter/comments.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/mod.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/groups.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/chain_member.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/simple_argument.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/print/binary_like_expression.rs`

Close criteria:

- `comments/attachment.md` is green
- the remaining failures in `transform/expressions/comments.md` are no longer comment-attachment or member-hop cases
- the local `MemberChain` shape reads like a thin AST adaptation layer over OXC
- head and tail grouping is no longer hidden inside oversized mixed helpers
- optional-boundary and inline-hop comment ownership is stable in both unit and transform tests

### Tranche 3. Chain Family Parity

This tranche starts only after the comment-attachment and chain-boundary debt is out of the way.

Primary local files:

- `language/formatter/src/format/chain/mod.rs`
- `language/formatter/src/format/chain/expression.rs`
- `language/formatter/src/format/chain/groups.rs`
- `language/formatter/src/format/chain/member.rs`
- `language/formatter/src/format/chain/simple_argument.rs`

Checklist:

- [ ] make `chain/expression.rs` orchestration-only where possible
- [ ] move extraction and normalized chain-part building into `chain/member.rs`
- [ ] keep `chain/groups.rs` focused on OXC-like head and tail grouping only
- [ ] delete local grouping heuristics that do not map to `member_chain`
- [ ] port or recreate the relevant OXC tests for head or tail splitting, optional calls, computed access, and comment-preserving member hops
- [ ] make chain-specific formatter unit tests and targeted integration fixtures pass

Primary OXC references:

- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/mod.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/groups.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/chain_member.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/utils/member_chain/simple_argument.rs`

Close criteria:

- the local `MemberChain` shape reads like a thin AST adaptation layer over OXC
- head and tail grouping is no longer hidden inside oversized mixed helpers
- optional-boundary and inline-hop comment ownership is stable in both unit and transform tests

### Tranche 4. Declarator And Assignment-Like Parity

This tranche closes the compensation layer that currently sits on top of call and chain.

Primary local files:

- `language/formatter/src/format/expression/declarator.rs`
- `language/formatter/src/format/operator/assign.rs`
- `language/formatter/src/format/declaration/assignment.rs`
- `language/formatter/src/format/expression/parentheses.rs`

Checklist:

- [ ] remove declarator carveouts that only exist because call and chain are still locally shaped
- [ ] port assignment-like shell ownership against OXC `assignment_like`
- [ ] keep parentheses decisions explicit and local instead of relying on wide helper lattices
- [ ] port or recreate OXC coverage for assignment chains, declarator rhs shaping, and grouped parentheses cases
- [ ] make the declarator and assignment formatter tests green

Primary OXC references:

- `~/symbol/oxc/crates/oxc_formatter/src/utils/assignment_like.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/parentheses/expression.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/print/variable_declaration.rs`

Close criteria:

- `format_declarator` no longer compensates for unfixed call or chain structure
- assignment-like layout selection is recognizable against OXC
- the remaining helpers in this family are either direct analogues or obvious local AST adaptation

### Tranche 5. Type, Binary, And Union Parity

This tranche closes the remaining high-risk JS and TS shell logic after the call and assignment path is stable.

Primary local files:

- `language/formatter/src/format/operator/binary.rs`
- `language/formatter/src/format/operator/type.rs`
- `language/formatter/src/format/operator/types.rs`
- `language/formatter/src/format/operator/union.rs`
- `language/formatter/src/format/declaration/signature.rs`

Checklist:

- [ ] keep ordinary binary layout aligned with OXC `binary_like_expression`
- [ ] keep type shell ownership aligned with OXC TS printers and parentheses logic
- [ ] remove local-only union or intersection comment attachment policy where OXC already has a clearer owner
- [ ] port or recreate OXC unit coverage for binary flattening, type shells, union or intersection comments, and signature wrapping
- [ ] make type and binary formatter tests green

Primary OXC references:

- `~/symbol/oxc/crates/oxc_formatter/src/print/binary_like_expression.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/print/union_type.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/print/intersection_type.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/print/parameters.rs`
- `~/symbol/oxc/crates/oxc_formatter/src/parentheses/ts_type.rs`

Close criteria:

- union and intersection comment ownership no longer depend on local seam helpers
- type shell helpers have clear upstream peers
- the remaining divergence is only Destack syntax and not legacy local policy

### Tranche 6. Collection, Property, And Module Ownership

This tranche cleans up the remaining comment and separator ownership families that are still red in the integration suite.

Primary local files:

- `language/formatter/src/format/collection/member.rs`
- `language/formatter/src/format/collection/property.rs`
- `language/formatter/src/format/expression/object.rs`
- `language/formatter/src/format/declaration/dependency.rs`
- `language/formatter/src/format/declaration/sequence.rs`
- `language/formatter/src/format/declaration/semicolon.rs`

Checklist:

- [ ] fix trailing-comment ownership for arrays, objects, class fields, import specifiers, and export specifiers
- [ ] remove local separator-comment compensation where OXC already has a stable owner
- [ ] port or recreate OXC tests for property trailing comments and module-specifier ownership
- [ ] make the relevant transform comment-boundary fixtures pass

Close criteria:

- collection and module ownership failures are no longer dominant in the transform suite
- the family no longer relies on comment-stealing behavior to keep outputs stable

### Tranche 7. Statement, Control, And Annotation Ownership

This tranche cleans up the remaining statement-boundary and annotation fallthrough after the expression shells are stable.

Primary local files:

- `language/formatter/src/format/expression/control.rs`
- `language/formatter/src/format/declaration/statement.rs`
- `language/formatter/src/format/declaration/sequence.rs`
- `language/formatter/src/format/annotation/sequence.rs`
- `language/formatter/src/format/annotation/trivia.rs`

Checklist:

- [ ] fix statement-tail comment ownership for break, continue, empty statements, switch cases, and related shells
- [ ] keep annotation and raw-comment handling on one formatter-side ownership model
- [ ] port or recreate focused tests for statement boundaries and annotation placement
- [ ] make the comment-boundary statement fixtures pass

Close criteria:

- statement-boundary comment failures are no longer a major integration bucket
- annotation and comment ownership does not require family-specific raw cursor hacks

### Tranche 8. Tree Discipline Pass

This tranche is intentionally last because it is not a literal OXC port target.

Primary local files:

- `language/formatter/src/format/tree/literal.rs`
- `language/formatter/src/format/tree/child.rs`
- `language/formatter/src/format/tree/argument.rs`
- `language/formatter/src/format/tree/attribute.rs`

Checklist:

- [ ] keep tree layout rules structurally disciplined and consistent with the rest of the formatter
- [ ] remove arbitrary expansion thresholds and ad hoc shell ownership
- [ ] keep the tree family tested through targeted formatter fixtures

Close criteria:

- tree formatting feels like a disciplined extension of the OXC-shaped formatter core
- tree-specific logic is local and does not distort the JS and TS parity path

## Working Family Map

### 1. Call-Like Expression Shaping

Audit status:

- checkpointed
- the family split and grouped-layout owners are now close enough to OXC to stop treating call-like formatting as the main active rewrite target

Primary files:

- `language/formatter/src/format/call/arguments.rs`
- `language/formatter/src/format/call/grouped.rs`
- `language/formatter/src/format/call/list.rs`

### 2. Comment Attachment And Member Chain Boundaries

Audit status:

- structurally suspect
- currently the highest-signal active implementation bucket
- attachment truth is now settled, but the formatter owners still do not follow it consistently

Primary files:

- `language/formatter/src/format/context/comment.rs`
- `language/formatter/src/format/expression/member.rs`
- `language/formatter/src/format/chain/expression.rs`
- `language/formatter/src/format/chain/groups.rs`
- `language/formatter/src/format/chain/member.rs`
- `language/formatter/src/format/operator/binary.rs`

### 3. Array And Collection Comment Ownership

Audit status:

- mixed
- some local support is justified, but a few longer helpers are still in the bad bucket

Primary files:

- `language/formatter/src/format/collection/member.rs`
- `language/formatter/src/format/collection/property.rs`
- `language/formatter/src/format/expression/object.rs`

### 4. Import And Export Ownership

Audit status:

- mostly support-layer and indirect-analogue work now
- still worth doing, but no longer the main structural risk

Primary files:

- `language/formatter/src/format/declaration/dependency.rs`
- `language/formatter/src/format/declaration/sequence.rs`
- `language/formatter/src/format/declaration/semicolon.rs`

### 5. Documentation And Annotation Ownership

Audit status:

- mostly support-layer
- body-level annotation truth is now settled
- still important for correctness, but no longer the main truth-setting task

Primary files:

- `language/formatter/src/format/annotation/sequence.rs`
- `language/formatter/src/format/annotation/trivia.rs`
- relevant declaration and member writers

### 6. Type Shell And Type Parameter Shaping

Audit status:

- structurally risky
- one of the highest bad-item buckets after call and chain

Primary files:

- `language/formatter/src/format/operator/type.rs`
- `language/formatter/src/format/operator/union.rs`
- `language/formatter/src/format/declaration/signature.rs`
- `language/formatter/src/format/operator/binary.rs`

### 7. Control And Statement Body Shaping

Audit status:

- mixed
- ownership and adjacency logic still need cleanup, but the family is not as structurally off as call or chain

Primary files:

- `language/formatter/src/format/expression/control.rs`
- `language/formatter/src/format/declaration/statement.rs`
- `language/formatter/src/format/declaration/sequence.rs`

### 8. Tree-Specific Layout Discipline

Audit status:

- separate
- not a literal OXC port target, but still needs a serious discipline pass

Primary files:

- `language/formatter/src/format/tree/literal.rs`
- `language/formatter/src/format/tree/child.rs`
- `language/formatter/src/format/tree/argument.rs`
- `language/formatter/src/format/tree/attribute.rs`

## Strategy

Use the suites to identify shared formatter families that still diverge.
Use the reviewed census to decide whether a local item is a real analogue, a support layer, or a real rewrite target.
Port whole families toward OXC, then use both suites to validate the port.
Do not chase isolated fixtures unless the family has already been structurally aligned.
Focus only on supported strict modern JS and TS surface when defining parity work.
Treat non JS and non TS Prettier suites as out of scope unless we explicitly decide otherwise.

The general loop for each family is:

1. Start from the reviewed items in `FORMATTER_MAP.md`.
2. Re-read the corresponding `oxc_formatter` family directly.
3. Remove local-only policy and rebuild rewrite-target items structurally.
4. Keep parser and trivia fixes upstream when the formatter is compensating for ownership problems.
5. Validate targeted `oxfmt`, targeted `prettier`, and full formatter tests.
6. Only then allow narrow point fixes inside the already-aligned family.

## Validation

Every family rewrite should validate at three levels.

### Local Formatter

- `cargo fmt -p destack_formatter`
- `cargo check -p destack_formatter`
- `cargo test -p destack_formatter -- --format=terse`

### Oxfmt

Use targeted conformance runs first, then broader runs as the family stabilizes.

- `cargo test --release -p destack_test --test conformance-formatter -- --oxfmt`

### Prettier

Use targeted conformance runs first, then broader runs as the family stabilizes.

- `cargo test --release -p destack_test --test conformance-formatter -- --prettier`

## Family Signoff Checklist

Each tranche is only closed when all of the following are true.

- [ ] the local file split matches clear upstream ownership boundaries
- [ ] the relevant upstream unit coverage has been ported or intentionally recreated locally
- [ ] the local formatter crate tests for the family are green
- [ ] the relevant filtered `destack_test --test formatter` cases for the family are green
- [ ] targeted `oxfmt` and `prettier` conformance checks were run for the family
- [ ] the map and plan were regenerated or updated after the checkpoint
- [ ] the remaining divergence is either Destack syntax or an explicitly documented unsupported case

## Status Discipline

Status files are only for unsupported or explicitly deferred cases.
They are not a substitute for parity work.

Before adding a new ignore:

1. confirm the syntax is outside supported strict modern JS and TS
2. confirm the failure is not just a formatter divergence
3. note the reason clearly in the status entry

## Working Notes

The formatter is in a better place than the old pre-parity generation, but it is not ready to declare victory.
The audit proved that some surviving families still look nothing like OXC internally, even when the broad family mapping is correct.
The next useful checkpoint is Tranche 2 of the execution sequence.
That tranche is the comment-attachment and chain-boundary rewrite, with chain, declarator, and binary fallout explicitly queued behind it.
