# Changelog

Versioning is based on [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
The changelog format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.50.0] - 2025-12-16

### Added

- Introduce linter with configurable rules and auto-fixing
- Support nominal interfaces
- Add `debugger` expression and statement
- Scaffold IDE/LSP queries for navigation and refactoring
- Inject builtin prelude automatically

### Changed

- Streamline and expand CLI
- Support basic desugar and elaborate phase
- Require `extension` keyword for extension declarations

## [0.49.0] - 2025-12-14

### Added

- Support circular module resolution
- Support basic extension types
- Support abstract declarations and const enums
- Add bind-level validation with dsconfig/tsconfig
- Resolve labelled statements

### Changed

- Defer most validation to analyze phase
- Split source and base, move config into workspace

### Fixed

- Avoid recursion error in infer_member_of_symbol
- Maintain stack size to avoid overflow on pathological ASTs
- Avoid deadlock in regex string dump
- Improve parser robustness for yield, throw, default imports

## [0.48.0] - 2025-12-06

### Added

- Add Cranelift codegen backend for native compilation
- Add bytecode interpreter ("Machine") for MIR execution
- Extend MIR with parser, naming, extern, managed memory
- Support basic type analysis and checking with structural typing
- Support labeled tuple-like type arguments
- Support sequence expressions and numeric keys

### Changed

- Change compiler to pull-based tasks
- Split Module by representation level (ModuleAst, ModuleDir)
- Follow TSX semantics for all tree literals
- Extend dsconfig with schema, runtime, and platform

### Fixed

- Improve parser robustness for TSX parsing
- Handle unterminated string literals and invalid escapes
- Prevent race condition in try_requeue_yielded
- Defer type string diagnostics to avoid deadlock

### Notes

- Conformance tests added for test262, biome, swc, and babel (~70% test262 pass rate)

## [0.47.4] - 2025-12-05

### Added

- Add `format` command to CLI
- Introduce reflection as a standalone language feature

### Changed

- Perform ASI (automatic semicolon insertion) in the parser
- Streamline compiler phases and task scheduling
- Set up new markdown-based testing harness

## [0.47.2] - 2025-12-04

### Changed

- Streamline syntax: merge tags into decorators, remove explicit `with`/context
- Rename scoped packages to top-level
- Split trees into separate feature
- Extend type analysis infrastructure

## [0.45.11] - 2025-12-03

Initial (public) release of Destack.

### Added

- Initial publication

### Notes

- Prior to 0.45.11, Destack used internal calendar versioning (YYYY.MM.DD.R). The minor version number is the repository age in months to reflect that.

[Unreleased]: https://github.com/destack-sh/destack/compare/v0.50.0...HEAD
[0.50.0]: https://github.com/destack-sh/destack/compare/v0.49.0...v0.50.0
[0.49.0]: https://github.com/destack-sh/destack/compare/v0.48.0...v0.49.0
[0.48.0]: https://github.com/destack-sh/destack/compare/v0.47.4...v0.48.0
[0.47.4]: https://github.com/destack-sh/destack/compare/v0.47.2...v0.47.4
[0.47.2]: https://github.com/destack-sh/destack/compare/v0.45.11...v0.47.2
[0.45.11]: https://github.com/destack-sh/destack/releases/tag/v0.45.11
