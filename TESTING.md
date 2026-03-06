# Testing

Our goal is a 100% bullet-proof stack and toolchain with the best possible performance: the Destack language, library, services, apps, and bridges:
1. Destack must never hardcrash
2. Destack must never fail in unexpected ways
3. Destack must be fast

## Gate Levels

Use the same gate model locally and in GitHub Actions.
Run `quick` during local iteration.
Run `commit` before every push.
Run `nightly` for deep validation.

| Level | Purpose | Local command |
|-------|---------|---------------|
| **Quick** | Fast, high-signal deterministic correctness | `just quick` |
| **Commit** | Blocking local and CI gate with additional deterministic assurance | `just commit` or `just precommit` |
| **Nightly** | Deep validation and release verification depth | `just nightly` |

Scoped lanes follow the same vocabulary.
Use `just language/quick`, `just language/commit`, `just language/nightly`, and the equivalent commands under `library/`, `service/`, `app/`, and `bridge/`.
Release reuses the `nightly` verification depth and then adds packaging, signatures, and publishing.
For the language stack, `quick` carries the core correctness suites.
`commit` currently reuses that deterministic lane.
`nightly` adds ecosystem canaries and stress.

## Reliability

The only way to ensure 100% reliability is to test everything, and test it thoroughly.

| Suite | Location | Description |
|-------|----------|-------------|
| **Smoke** | [language/test/fixtures/smoke/](language/test/fixtures/smoke/) | Parser and compiler don't crash on any input |
| **Codegen** | [language/test/fixtures/codegen/](language/test/fixtures/codegen/) | Codegen output matches expected snapshots |
| **Specification** | [language/test/fixtures/specification/](language/test/fixtures/specification/) | MDTest-driven type checking and diagnostics |
| **Query** | [language/test/fixtures/query/](language/test/fixtures/query/) | MDTest-driven IDE/LSP queries (goto definition, completion, rename) |
| **Conformance** | [language/test/fixtures/parser/conformance/](language/test/fixtures/parser/conformance/) | Parser conformance against established test suites |
| **Formatter** | [language/test/fixtures/formatter/](language/test/fixtures/formatter/) | Format roundtrip stability |
| **Resolver** | [language/test/fixtures/resolver/](language/test/fixtures/resolver/) | Module resolution (node_modules, pnpm, yarn, tsconfig paths) |
| **Interop canary** | [language/test/fixtures/ecosystem/](language/test/fixtures/ecosystem/) | Curated TS-first Node, backend, and tooling packages |
| **Stress** | [language/test/fixtures/stress/](language/test/fixtures/stress/) | Scale limits: large files, many modules, deep nesting |
| **Fuzz** | [language/parser/fuzz/](language/parser/fuzz/), [language/formatter/fuzz/](language/formatter/fuzz/) | Random input exploration |

Run these commands from the repository root.

```bash
# primary lanes
just quick
just commit
just nightly

# individual language suites
just language/ecosystem-fetch
just language/test-smoke
just language/test-codegen
just language/test-specification
just language/test-query
just language/test-parser-conformance
just language/test-formatter
just language/test-resolver
just language/test-ecosystem
just language/generate-stress
just language/test-stress
```

```bash
just bench                       # parser benchmarks
just language/bench-compiler     # compiler benchmarks
just language/bench-lexer        # lexer only
just language/bench-parser       # parser only
```

```bash
just fuzz                        # all fuzzers
just language/fuzz-lexer         # lexer only
just language/fuzz-parser        # parser only
just language/fuzz-formatter     # formatter only
```

## Fuzzing Expansion

We plan to expand fuzz coverage beyond lexer and formatter targets.
The current parser target is tracked in [language/parser/fuzz/README.md](language/parser/fuzz/README.md).
New targets will be added as dedicated jobs once corpus quality and runtime budgets are stable.
