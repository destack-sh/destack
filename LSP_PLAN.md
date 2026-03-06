# LSP Plan

This document tracks the final shape of the `language/test` applied LSP harness.
This document is no longer a migration log.
This document records the live architecture, the current coverage bar, and the remaining work that still matters.

## Final Model

The canonical applied LSP fixture container is markdown.
The canonical source block shape is `ds` or `ds:path/to/file.ds`.
The canonical expectation block shape is `lsp <kind> [args...]`.
Markers stay inline in source as `/*name*/`.
Ranges stay inline in source as `[|...|]`.
The harness is editor shaped on top and LSP shaped underneath.
The harness drives the real in process `destack_lsp` server.

We do not keep `.fourslash` files.
We do not keep `// @Filename:` or `////` outer syntax.
We do not keep comment directive parsers like `@Scenario` or `@...Expected`.
We do not keep support pseudo files like `expected.ds` or `expected.workspace`.

## Responsibility Split

1. `language/test`.
This crate owns the canonical applied LSP fixture corpus and native harness.

2. `language/service`.
This crate owns lower level query and refactor correctness without protocol or editor indirection.

3. `service/lsp`.
This crate owns protocol mapping, lifecycle, cancellation, progress, diagnostics sequencing, and read versus write correctness.

4. `bridge/vscode` and `bridge/zed`.
These crates own activation, command wiring, packaging, grammar, transport smoke, and editor specific seams only.

## Fixture Model

Each applied LSP case is an `MdTestCase`.
Each case contains one or more `ds` or `ds:path` blocks for virtual workspace files.
Each case may contain markers and ranges inside those source blocks.
Each case contains one or more `lsp ...` blocks for requests, expectations, or explicit procedural metadata.
Each case may contain `json:path` blocks only when the workspace genuinely needs extra data files.

The fixture families on disk are:

- `language/test/fixtures/lsp/assist/`
- `language/test/fixtures/lsp/diagnostic/`
- `language/test/fixtures/lsp/navigation/`
- `language/test/fixtures/lsp/refactor/`
- `language/test/fixtures/lsp/lifecycle/`

The fixture organization should follow the local `query` style, not tsserver's historical flat file sprawl.
That means capability files with multiple cases, meaningful prose headings, and exact snapshots.

## Block Grammar

The supported `lsp` block families should stay small and explicit.
Each family should have one obvious parser and one obvious runner target.

### Request and expectation blocks

- `lsp definition <target>`
- `lsp declaration <target>`
- `lsp type_definition <target>`
- `lsp references <target>`
- `lsp implementation <target>`
- `lsp hover <target>`
- `lsp quick_info <target>`
- `lsp completion <target>`
- `lsp completion_resolve`
- `lsp signature_help <target>`
- `lsp prepare_rename <target>`
- `lsp rename <target> <new_name>`
- `lsp document_symbols`
- `lsp workspace_symbols <query>`
- `lsp document_link`
- `lsp document_link_resolve`
- `lsp code_actions <target>`
- `lsp code_action_resolve <target>`
- `lsp code_lens`
- `lsp code_lens_resolve`
- `lsp document_formatting`
- `lsp range_formatting <target>`
- `lsp on_type_formatting <target> <character>`
- `lsp folding_range`
- `lsp inlay_hint`
- `lsp semantic_tokens`
- `lsp semantic_tokens_range <target>`
- `lsp semantic_tokens_delta`
- `lsp document_diagnostic <target>`
- `lsp workspace_diagnostic`
- `lsp workspace_diagnostic_partial`
- `lsp selection_range <target>`
- `lsp call_hierarchy_incoming <target>`
- `lsp call_hierarchy_outgoing <target>`
- `lsp type_hierarchy_supertypes <target>`
- `lsp type_hierarchy_subtypes <target>`
- `lsp current_file`

### Procedural metadata blocks

- `lsp scenario <kind>`
- `lsp open_text <path>`
- `lsp execute_command <command>`
- `lsp format_options`
- `lsp cancellation_policy`

Expectation blocks should read like `query` output, not like raw transport dumps.
Complex payloads should use readable multiline records.
Expectation grammars should be capability specific and exact.

## Harness Shape

The top level `language/test/src/lsp/` layout is:

- `mod.rs`
- `main.rs`
- `fixture/`
- `harness/`
- `interface/`
- `runner/`

### Fixture module

- `fixture/mod.rs`: re exports only.
- `fixture/core.rs`: `LspFixture`, `LspExpectations`, marker and range types, and capability specific expectation data.
- `fixture/parse.rs`: parse one `MdTestCase` into one native LSP case.
- `fixture/expect.rs`: parse capability specific expectation content from `lsp ...` blocks.

### Harness module

- `harness/mod.rs`
- `harness/driver.rs`
- `harness/state.rs`
- `harness/edit.rs`
- `harness/normalize.rs`
- `harness/verify.rs`
- `harness/suite.rs`

### Interface module

The public native facade split follows TypeScript closely:

- `Test`
- `GoTo`
- `Verify`
- `VerifyNegatable`
- `Edit`
- `Format`
- `Cancellation`
- `Debug`

### Runner module

The runner dispatches by parsed expectation kind, not by file name or fixture extension.

- `runner/mod.rs`
- `runner/core.rs`
- `runner/navigation.rs`
- `runner/assists.rs`
- `runner/refactor.rs`
- `runner/symbols.rs`
- `runner/hierarchy.rs`
- `runner/tokens.rs`
- `runner/diagnostics.rs`
- `runner/edits.rs`
- `runner/cancellation.rs`
- `runner/lifecycle.rs`
- `runner/commands.rs`

## TypeScript Parity

The reference files are:

- `~/symbol/TypeScript/src/harness/fourslashImpl.ts`
- `~/symbol/TypeScript/src/harness/fourslashInterfaceImpl.ts`
- `~/symbol/TypeScript/src/harness/runnerbase.ts`

The harness is intentionally aligned with TypeScript at the facade and scenario level.
The harness is not intended to port all historical tsserver helper surface blindly.

### Live parity

The following facade families exist with the same top level role in Rust:

- `Test`
- `GoTo`
- `Verify`
- `VerifyNegatable`
- `Edit`
- `Format`
- `Cancellation`
- `Debug`

The following TypeScript facade is intentionally absent:

- `Config`

### Intentional non ports

The following TypeScript helpers are intentionally not targets for the Destack applied LSP harness today:

- compiler host helpers like `symbolsInScope` and `setTypesRegistry`
- plugin and inferred project config helpers
- JSX and brace completion helpers
- TS specific code fix inventory helpers
- TS emitter and baseline helpers
- navigation tree and outlining debug baselines

These are not meaningful unless the Destack LSP or editor product surface actually grows in that direction.

### Product level gaps

The real remaining product level gaps compared to tsserver style editor capability are:

1. linked editing.
This is a real protocol capability gap rather than a harness gap.

2. paste edits and prepare paste edits.
These are product surface gaps, not mdtest harness gaps.

3. a direct `applyRefactor` style helper.
If we ever add one, it should round trip through real code actions or rename application rather than becoming a fake local shortcut.

## Advertised Capability Coverage

The real completeness bar is the live `initialize` surface in `service/lsp/src/server/server.rs`.

### Applied `language/test` coverage

- [x] `textDocument/hover`
- [x] `textDocument/definition`
- [x] `textDocument/declaration`
- [x] `textDocument/typeDefinition`
- [x] `textDocument/references`
- [x] `textDocument/documentSymbol`
- [x] `workspace/symbol`
- [x] `textDocument/documentHighlight`
- [x] `textDocument/completion`
- [x] `completionItem/resolve`
- [x] `textDocument/signatureHelp`
- [x] `textDocument/semanticTokens/full`
- [x] `textDocument/semanticTokens/full/delta`
- [x] `textDocument/semanticTokens/range`
- [x] `textDocument/diagnostic`
- [x] `workspace/diagnostic`
- [x] `textDocument/formatting`
- [x] `textDocument/rangeFormatting`
- [x] `textDocument/onTypeFormatting`
- [x] `textDocument/foldingRange`
- [x] `textDocument/selectionRange`
- [x] `textDocument/documentLink`
- [x] `documentLink/resolve`
- [x] `textDocument/rename`
- [x] `textDocument/prepareRename`
- [x] `textDocument/codeAction`
- [x] `codeAction/resolve`
- [x] `textDocument/codeLens`
- [x] `codeLens/resolve`
- [x] `textDocument/inlayHint`
- [x] `textDocument/implementation`
- [x] `callHierarchy/prepare`, `callHierarchy/incomingCalls`, and `callHierarchy/outgoingCalls`
- [x] `textDocument/prepareTypeHierarchy`, `typeHierarchy/supertypes`, and `typeHierarchy/subtypes`
- [x] `workspace/executeCommand`
- [x] `didOpen`, `didChange`, `didSave`, and `didClose`
- [x] progress and cancellation flows for long running requests

### `service/lsp` owned protocol seams

The following protocol seams remain primarily owned by `service/lsp` tests:

- [x] workspace folder registration and scoping
- [x] watched file registration
- [x] create, rename, and delete file operation fanout
- [x] multi root protocol edge cases

There are currently no known uncovered capabilities advertised by `service/lsp` that should belong in the applied `language/test` harness.

## Immediate Remaining Work

The largest remaining harness weakness is multi step sequencing.
Today some edit then query then edit then query flows still live too much in runner code.
That is weaker than tsserver's explicit ordered test scripting model.

The next structural step is to make ordered multi step sequences first class in markdown.

### Step model

Add a typed `LspStep` list to `LspFixture`.
Parse ordered `lsp` step blocks in source order.
Execute them sequentially in a generic step runner.
Keep the syntax explicit and small.
Do not add loops, branching, or a mini programming language.

### Candidate step families

- editor steps like `open`, `close`, `save`, `go_to_marker`, `go_to_file`, `select`, `insert`, `replace_selection`, `replace`, `backspace`, `delete_at_caret`, `delete_line`, and `replace_line`
- sync steps like `wait_mutation_idle`, `wait_document_diagnostic`, `wait_workspace_diagnostic`, `wait_progress`, `cancel_request`, and `cancel_progress`
- request and assert steps like `expect_definition`, `expect_hover`, `expect_completion`, `expect_document_diagnostic`, `expect_workspace_diagnostic`, and `expect_current_file`
- command steps like `execute_command <name>`

### Step work checklist

- [ ] Add `LspStep` to `language/test/src/lsp/fixture/core.rs`.
- [ ] Parse ordered step blocks in `language/test/src/lsp/fixture/parse.rs`.
- [ ] Add exact step payload parsers in `language/test/src/lsp/fixture/expect.rs` where needed.
- [ ] Replace bespoke multi step runner branches with generic step execution in `language/test/src/lsp/runner/core.rs`.
- [ ] Port the current lifecycle edit and command flows to ordered steps.
- [ ] Keep simple single request cases structural and implicit where possible.

## Secondary Remaining Work

- [ ] Re audit the bridge crates to confirm no semantic LSP corpus drifted back out of `language/test`.
- [ ] Simplify the simplest navigation fixtures further so common cases need no procedural metadata.
- [ ] Keep fixture prose and grouping aligned with the `query` suite as the corpus grows.

## Validation

- [x] `cargo fmt -p destack_test -p destack_lsp`
- [x] `cargo test -p destack_test --test lsp`
- [x] `cargo test -p destack_test --test query`
- [x] `cargo test -p destack_lsp`

## Current Status

The live implementation now uses markdown mdtest discovery in `language/test/src/lsp/harness/suite.rs`.
The live implementation now parses `MdTestCase` values directly in `language/test/src/lsp/fixture/parse.rs`.
The live implementation now uses inline `lsp ...` snapshots for formatting, diagnostics, semantic tokens, hierarchy, code actions, commands, and editor overlays.
The live implementation now covers the full currently advertised applied LSP surface.
The remaining work is about making procedural sequencing more explicit and keeping the surrounding bridge split clean.
