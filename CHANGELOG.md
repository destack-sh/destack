# Changelog

This file tracks notable release changes for Destack.
Entries are generated from conventional commit history during release preparation.

## [0.55.4] - 2026-03-04

_Changes since v0.50.0._

### Features
- feat(dev(all)): unify multi-registry publish flow and live entrypoints
- feat(bridge): scaffold runtime clients with core/capi
- feat(language/runtime/platform): extend win32 display backend
- feat(language/builtin): extend display module surface
- feat(language/runtime/platform): scaffold basic wind32 display backend
- feat(language/parser): allow newline-prefixed postfix static arguments
- feat(language/builtin): extend display, also with backends
- feat(language/runtime/platform): implement basic memory bindings
- feat(languaeg/runtime): implement basic ipc module
- feat(language/runtime): implement basic tty module
- feat(language/builtin): extend tty platform module surface
- feat(language/source): add Span::gap_to for ordered trivia gaps
- feat(language/runtime/fs): implement watch, support bytes paths more fully
- feat(language/runtime/net): implement packet/route net bindings
- feat(language/fir): support trim_trailing_line_whitespace
- feat(language/dir): model type freshness explicitly
- feat(language): support import.meta.target
- feat(language/runtime): make os.credential mechanism and requirements more explicit
- feat(language/runtime): support basic os.credentials submodule
- feat(language/runtime/crypto): implement expanded crypto backend and host lanes
- feat(language/builtin): extend crypto bindings
- feat(language/runtime/crypto): support basic software/hardware backed crypto bindings
- feat(language/runtime): implement basic macos crypto backend, scaffold other hosts
- feat(language/formatter): parenthesize extends expressions in declaration
- feat(language/formatter): parenthesize super expressions in declaration
- ... and 344 more

### Fixes
- fix(bridge): use modern pub.dev credentials path for dart publish
- fix(language/compiler/elaborate): align lower with uninitialized let output
- fix(language/parser): disambiguate pattern / must precedence
- fix(language/formatter): preserve explicit this parameters in method signatures
- fix(language/parser): disambiguate cast comparisons from static arguments
- fix(language/formatter): stabilize declaration, parentheses, and ignore-directive paths
- fix(language/formatter): align annotation seam ownership for call and ternary
- fix(language/formatter): stabilize type alias break decisions around non-blank annotations
- fix(language/formatter): stabilize member seam boundary rendering and call separator routing
- fix(all): update READMEs
- fix(language/formatter): normalize JSX text boundary spacing
- fix(language/formatter): check delimiter_interior_container_owner
- fix(language/formatter): stabilize seam ownership and chain idempotence
- fix(language/formatter): stabilize literal and expression idempotence
- fix(language/parser): protected stack growth in recursive blocks
- fix(language/formatter): normalize annotation seam ownership across boundaries
- fix(language/runtime): harden os.info signed-to-u64 conversions
- fix(language): robustify template span inference and infer materialization
- fix(language): robustify overload receiver dispatch and refresh failure baselines
- fix(language): robustify associated projections and type-index disambiguation
- fix(language/parser): clamp dynamic argument/value spans before separators
- fix(language/compiler/analyze): defer relation diagnostics and tighten member/operator fallback reporting
- fix(language/compiler/analyze): preserve symbolic static arguments through projection convergence
- fix(language/formatter): align call/expression/operator idempotence
- fix(language/formatter): normalize annotation ownership and seam routing
- ... and 222 more

### Refactors
- refactor(all): mark old aliases as legacy explicitly
- refactor(language/compiler/elaborate): centralize elaborate state threading
- refactor(language/compiler): reorganize elaborate tasks and tests
- refactor(language/formatter): centralize call layout and statement formatting phases
- refactor(language/formatter): decompose annotation expression/operator/render pipelines
- refactor(language/formatter): split annotation attachment into seam-owner phases
- refactor(language/formatter): streamline annotation and context handling
- refactor(language/runtime): introduce explicit runtime/platform 'state', generalize PlatformDiagnosticStore
- refactor(language/runtime): clean up platform plumbing after abi migration
- refactor(language/runtime): move native abi primitives into runtime core
- refactor(language/runtime): centralize host cleanup hooks and message pumping
- refactor(language/runtime): centralize and harden windows core callback/com plumbing
- refactor(language/runtime): generalize/extract more platform core (incl. DLL stuff)
- refactor(languaeg/formatter): streamline 'dropped' naming
- refactor(language/formatter): simplify annotation declaration and expression seam ownership
- refactor(language/formatter): align annotation seams and boundary ownership
- refactor(language/formatter): split annotation attachment for member seam ownership
- refactor(language/runtime): centralize RuntimeOptions/RuntimeState for platform modules
- refactor(language/runtime): reorganize display backends
- refactor(language/runtime): regenerate/rewire more bindings with new more idiomatic TS++ shape
- refactor(language/runtime): use proper source docs for generated ABI stuff
- refactor(language/runtime): use heap::Value directly, streamline policy stuff
- refactor(language/runtime): reify Policy, Rules, Effect, Hooks, World
- refactor(language/runtime): split out World, policy stuff
- refactor(language/formatter): split annotation attachment for JSX closing-tag seams
- ... and 365 more

### Docs
- docs(all): update/clarify specs
- docs(all): update/clarify specs
- docs(all): update README
- docs(all): update README
- docs(all): update specs
- docs(all): update specs
- docs(all): update specs
- docs(all): update README
- docs(all): update README
- docs(all): update README
- docs(all): update README
- docs(all): update README
- docs(all): update README
- docs(all): update README
- docs(language/runtime): update documentation
- docs(all): add basic brand assets
- docs(all): add testing commands to all readmes
- docs(all): update specification
- docs(all): update specification
- docs(all): update READMEs
- docs(all): update specs
- docs(all): update specs
- docs(language): align license
- docs(*): untrack .vscode

### Tests
- test(language/compiler/elaborate): reorganize extend elaborate test coverage
- test(language/formatter): expand idempotence fixture coverage
- test(language): export extensions explicitly, correct class initialisiers
- test(language): expand spec tests (lots more combinatorial stuff)
- test(language): expand tsc parity coverage for satisfies and widening
- test(language): increase spec test timeout for contended runs
- test(language/runtime): setup symmetrical tests for hosts
- test(language/runtime/crypto): strengthen backend conformance coverage
- test(language/runtime/audio): allow unsupported audio backend capabilities
- test(language/formatter): add roundtrip regressions for seams and postfix stability
- test(language/formatter): ensure compact comments
- test(platform/lsp): introduce test harness
- test(platform/daemon): streamline test harness
- test(platform/cli): streamline test harness
- test(language/service): streamline test harness
- test(language/runtime): generate test harness mappings
- test(language/workspace): skip cache tests for now
- test(language/formatter): codify annotation behavior more precisely
- test(language/vm): update closure tests
- test(language/formatter): instrument formatter corpi better
- test(language/linter): update linter snapshot formatting
- test(language/formatter): align formatter expectations with prettier-ish standards
- test(language/linter): update linter with new IR/formatting expectations
- test(language/workspace): expand language query test coverage
- test(language): harden spec tests
- ... and 61 more

### Chores
- chore(all): bump version to 0.55.4
- dev(all): align version tooling and publish auth
- chore(language): regenerate dsconfig schema
- dev(all): use symbol.industries for maven
- dev(all): set up local env symlinks
- chore(all): reformat .ds files
- chore(ci(all)): unify toolchain setup and workflow policy checks
- chore(language/runtime): regenerate platform bindings with updated generator
- chore(language/runtime/platform): regenerate display bindings
- chore(language): streamline toolchain scripts and runtime targets
- dev(all): rename VERSION->VERSION.txt, bump it
- chore(language/formatter): rebaseline and widen prettier/formatter conformance target (:c)
- chore(language/formatter): ignore prettier single type test (oxfmt conflict)
- chore(language/test): break out formatter conformance by 2 layer category
- chore(all): update READMEs
- chore(all): update READMEs
- chore(all): update READMEs
- chore(all): update READMEs
- chore(all): update READMEs
- chore(all): update READMEs
- chore(all): update READMEs
- chore(all): update READMEs
- chore(language/mir): scaffold more explicit itab/vtable storage
- chore(language/runtime): scaffold display backends
- chore(language/runtime): normalize unix tty numeric projections
- ... and 498 more

### Other
- ci(all): setup cross-platform targets
- ci(all): setup Android/NDK stuff for CI
- perf(language/parser): tighten ParserOptions, add better fast paths
- perf(language/parser): optimize trivia / lookahead construction
- perf(language/parser): pack ParserOptions more tightly, introduce some hot paths
- ci(all): streamline gates
- perf(language/parser): (try to) optimize trivia attachment
- release(all): bump patch version
- ci(all): stabilize CI
- ci(all): set up basic github CI
- perf(language/parser): streamline lexer/scanner state tracking
- perf(language/parser): streamline lexer/scanner state tracking
- perf(language/parser): pass ParserOptions explicitly where sensible, prefer direct peek checks (without err path)
- perf(language/parser): cache token keywords
- perf(language/parser): prefer is_* peeking
- release(all): bump minor
- perf(language/parser): add hot paths for simple lambdas and declarators
- perf(language/parser): improve lookahead splits
- perf(language/parser): remember line terminators
- perf(language/parser): prefer is_* over peek_* with Result, avoid redundant rebuild on rewind/restore
- perf(language/for): allocate / build strings more intelligently
- perf(language/formatter): add some fast paths for common expressions
- perf(language/formatter): reduce best_fitting usage
- perf(language/formatter): extend analysis caches, reduce best_fitting IR
- perf(language/formatter): cache trivia/derived CST info stuff
- ... and 52 more

