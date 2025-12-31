# Testing

Our goal is a 100% bullet-proof stack and toolchain with the best possible performance: the Destack language, libraries and platform:
 1. Destack must never hardcrash
 2. Destack must never fail in unexpected ways
 3. Destack must be fast

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
just language/bench-lexer        # lexer only
just language/bench-parser       # parser only
```

```bash
just fuzz                       # all fuzzers
just language/fuzz-lexer         # lexer only
just language/fuzz-parser        # parser only
just language/fuzz-formatter     # formatter only
```
