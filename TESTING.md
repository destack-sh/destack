# Testing

Our goal is 100% bullet-proof reliability at the best possible performance: the Destack language, libraries and platform must never hardcrash, and must never fail (gracefully) in unexpected ways.

## Reliability

The only way to ensure 100% reliability is to test everything, and test it thoroughly:
| Suite | Location | Description |
|-------|----------|-------------|
| **Smoke** | [language/test/fixtures/smoke/](language/test/fixtures/smoke/) | Parser and compiler don't crash on any input |
| **Codegen** | [language/test/fixtures/codegen/](language/test/fixtures/codegen/) | Codegen output matches expected snapshots |
| **MDTest** | [language/test/fixtures/mdtest/](language/test/fixtures/mdtest/) | Markdown-driven type checking and diagnostics |
| **Conformance** | [language/test/fixtures/conformance/](language/test/fixtures/conformance/) | Parser conformance against test262, babel, swc, biome |
| **Formatter** | [language/test/fixtures/formatter/](language/test/fixtures/formatter/) | Format roundtrip stability |
| **Resolver** | [language/test/fixtures/resolver/](language/test/fixtures/resolver/) | Module resolution (node_modules, pnpm, yarn, tsconfig paths) |
| **Fuzzing** | [language/parser/fuzz/](language/parser/fuzz/) | Parser tokenizer fuzzing (~2,100 corpus entries) |

```bash
# all tests
just test

# individual suites
just test-smoke
just test-codegen
just test-mdtest
just test-conformance
just test-formatter

# fuzzing
just fuzz-parser          # 5 minutes (default)
just fuzz-parser 60       # 1 minute
just fuzz-ci              # brief CI run
```

## Performance

And the only way to ensure performance is to benchmark everything, and benchmark continuously.

| Suite | Location | Description |
|-------|----------|-------------|
| **Lexer** | [language/parser/benches/](language/parser/benches/) | Tokenizer throughput (lines/sec) |
| **Parser** | [language/parser/benches/](language/parser/benches/) | Parser throughput (lines/sec) |

```bash
just bench-parser         # all parser benchmarks
just bench-lex            # lexer only
just bench-parse          # parser only
```
