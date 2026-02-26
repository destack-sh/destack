# Testing

Our goal is a 100% bullet-proof stack and toolchain with the best possible performance: the Destack language, library, services, apps, and bridges:
1. Destack must never hardcrash
2. Destack must never fail in unexpected ways
3. Destack must be fast

## Gate Levels

Use the same gate model locally and in GitHub Actions.
Run blocking gates before every push.
Run thorough suites when validating larger or riskier changes.

| Level | Purpose | Local command |
|-------|---------|---------------|
| **Blocking CI** | Required for PR merge | `just precommit` (or `just ci`) |
| **Scoped blocking CI** | Faster local iteration in one area | `just language/ci`, `just library/ci`, `just service/ci`, `just app/ci`, `just bridge/ci` |
| **Full local matrix** | Broader validation before larger changes | `just test` |
| **Nightly depth** | Heavy suites and deeper regression detection | Covered by `.github/workflows/nightly.yml` |

## Reliability

The only way to ensure 100% reliability is to test everything, and test it thoroughly.

| Suite | Location | Description |
|-------|----------|-------------|
| **Smoke** | [language/test/fixtures/smoke/](language/test/fixtures/smoke/) | Parser and compiler don't crash on any input |
| **Codegen** | [language/test/fixtures/codegen/](language/test/fixtures/codegen/) | Codegen output matches expected snapshots |
| **Specification** | [language/test/fixtures/specification/](language/test/fixtures/specification/) | MDTest-driven type checking and diagnostics |
| **Query** | [language/test/fixtures/query/](language/test/fixtures/query/) | MDTest-driven IDE/LSP queries (goto definition, completion, rename) |
| **Conformance** | [language/test/fixtures/conformance/](language/test/fixtures/conformance/) | Parser conformance against established test suites |
| **Formatter** | [language/test/fixtures/formatter/](language/test/fixtures/formatter/) | Format roundtrip stability |
| **Resolver** | [language/test/fixtures/resolver/](language/test/fixtures/resolver/) | Module resolution (node_modules, pnpm, yarn, tsconfig paths) |
| **Ecosystem** | [language/test/fixtures/ecosystem/](language/test/fixtures/ecosystem/) | Real-world package parsing |
| **Stress** | [language/test/fixtures/stress/](language/test/fixtures/stress/) | Scale limits: large files, many modules, deep nesting |
| **Fuzz** | [language/parser/fuzz/](language/parser/fuzz/), [language/formatter/fuzz/](language/formatter/fuzz/) | Random input exploration |

Run these commands from the repository root.

```bash
# all tests
just test

# individual language suites
just language/ecosystem-fetch
just language/test-smoke
just language/test-codegen
just language/test-specification
just language/test-query
just language/test-conformance
just language/test-formatter
just language/test-resolver
just language/test-ecosystem
just language/generate-stress
just language/test-stress
```

```bash
just bench                      # parser benchmarks
just language/bench-compiler     # compiler benchmarks
just language/bench-lexer        # lexer only
just language/bench-parser       # parser only
```

```bash
just fuzz                       # all fuzzers
just language/fuzz-lexer         # lexer only
just language/fuzz-parser        # parser only
just language/fuzz-formatter     # formatter only
```

## Fuzzing Expansion

We plan to expand fuzz coverage beyond lexer and formatter targets.
The current parser target is tracked in [language/parser/fuzz/README.md](language/parser/fuzz/README.md).
New targets will be added as dedicated jobs once corpus quality and runtime budgets are stable.
