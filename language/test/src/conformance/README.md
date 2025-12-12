# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Total |  Rate   |
|:---------|-------:|-------:|------:|--------:|
| babel    |   437  |   283  |   720 |  60.69% |
| biome    |   393  |   244  |   637 |  61.70% |
| swc      |   390  |   148  |   538 |  72.49% |
| test262  |  4204  |  1159  |  5363 |  78.45% |
|----------|--------|--------|-------|---------|
| total    |  5424  |  1834  |  7258 |  74.73% |

Total Blended Pass Rate: **74.73%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| arrow-function       |    12  |    10  |    22 |  54.55% |
| assert-predicate     |     7  |     3  |    10 |  70.00% |
| assign               |     1  |     2  |     3 |  33.33% |
| async-call           |     0  |     1  |     1 |   0.00% |
| basic                |    29  |     9  |    38 |  76.32% |
| binary-expression    |     2  |     0  |     2 | 100.00% |
| cast                 |    30  |    12  |    42 |  71.43% |
| catch-clause         |     1  |     0  |     1 | 100.00% |
| class                |    47  |    50  |    97 |  48.45% |
| const                |     1  |     3  |     4 |  25.00% |
| declare              |    12  |    15  |    27 |  44.44% |
| decorators           |     1  |     1  |     2 |  50.00% |
| disallow-jsx-ambiguity |     2  |     1  |     3 |  66.67% |
| dts                  |     3  |     2  |     5 |  60.00% |
| enum                 |    13  |     0  |    13 | 100.00% |
| errors               |    19  |     9  |    28 |  67.86% |
| expect-plugin        |     0  |     3  |     3 |   0.00% |
| explicit-resource-management |     0  |     1  |     1 |   0.00% |
| exponentiation       |     1  |     2  |     3 |  33.33% |
| export               |    11  |     3  |    14 |  78.57% |
| function             |     5  |     8  |    13 |  38.46% |
| html-entities        |     3  |     1  |     4 |  75.00% |
| import               |     9  |    14  |    23 |  39.13% |
| interface            |    30  |    17  |    47 |  63.83% |
| legacy-decorators    |     1  |     1  |     2 |  50.00% |
| module-namespace     |    15  |     3  |    18 |  83.33% |
| optional-chaining    |     1  |     0  |     1 | 100.00% |
| regression           |    22  |     3  |    25 |  88.00% |
| scope                |    30  |    36  |    66 |  45.45% |
| static-blocks        |    14  |     6  |    20 |  70.00% |
| tsx                  |     4  |     4  |     8 |  50.00% |
| type-alias           |     5  |     2  |     7 |  71.43% |
| type-arguments       |    22  |    11  |    33 |  66.67% |
| type-arguments-bit-shift-left-like |     3  |     6  |     9 |  33.33% |
| type-only-import-export-specifiers |    14  |    10  |    24 |  58.33% |
| types                |    63  |    32  |    95 |  66.32% |
| types-arrow-function |     3  |     0  |     3 | 100.00% |
| variable-declarator  |     1  |     2  |     3 |  33.33% |
|----------------------|--------|--------|-------|---------|
| total                |   437  |   283  |   720 |  60.69% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| error                |   177  |   125  |   302 |  58.61% |
| ok                   |   216  |   119  |   335 |  64.48% |
|----------------------|--------|--------|-------|---------|
| total                |   393  |   244  |   637 |  61.70% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| amaro-194            |     0  |     1  |     1 |   0.00% |
| arrow-function       |     8  |     7  |    15 |  53.33% |
| basic                |    49  |    14  |    63 |  77.78% |
| case1                |     1  |     0  |     1 | 100.00% |
| cast                 |    11  |     5  |    16 |  68.75% |
| class                |    19  |    20  |    39 |  48.72% |
| const                |     1  |     0  |     1 | 100.00% |
| custom               |    42  |    14  |    56 |  75.00% |
| declare              |     5  |     0  |     5 | 100.00% |
| decorators           |     0  |     2  |     2 |   0.00% |
| deno                 |     0  |     1  |     1 |   0.00% |
| deno-1671            |     1  |     0  |     1 | 100.00% |
| deno-8925            |     2  |     0  |     2 | 100.00% |
| deno-9620            |     1  |     0  |     1 | 100.00% |
| deno-discord         |     2  |     0  |     2 | 100.00% |
| enum                 |    11  |     0  |    11 | 100.00% |
| eof-issue            |     1  |     0  |     1 | 100.00% |
| errors               |    15  |    10  |    25 |  60.00% |
| es2019               |     0  |     1  |     1 |   0.00% |
| estree-compat        |     1  |     0  |     1 | 100.00% |
| export               |     5  |     0  |     5 | 100.00% |
| export-default-interface |     1  |     0  |     1 | 100.00% |
| function             |     4  |     2  |     6 |  66.67% |
| import               |     1  |     7  |     8 |  12.50% |
| import-assertions    |     7  |     0  |     7 | 100.00% |
| instantiation-expr   |     6  |     2  |     8 |  75.00% |
| interface            |    16  |     0  |    16 | 100.00% |
| issue                |    92  |    44  |   136 |  67.65% |
| meta-property        |     3  |     0  |     3 | 100.00% |
| module-namespace     |     9  |     0  |     9 | 100.00% |
| next                 |     0  |     2  |     2 |   0.00% |
| nullish-coalescing-operator |     7  |     0  |     7 | 100.00% |
| object               |     2  |     0  |     2 | 100.00% |
| optional-chaining    |    16  |     2  |    18 |  88.89% |
| regression           |     4  |     0  |     4 | 100.00% |
| stack-overflow       |     1  |     0  |     1 | 100.00% |
| stack-size           |     1  |     0  |     1 | 100.00% |
| stc                  |     2  |     0  |     2 | 100.00% |
| template-literal-type |     1  |     0  |     1 | 100.00% |
| top-level-await      |     1  |     0  |     1 | 100.00% |
| ts-import-type       |     1  |     0  |     1 | 100.00% |
| tsx                  |     1  |     2  |     3 |  33.33% |
| type-alias           |     5  |     0  |     5 | 100.00% |
| type-arguments       |     5  |     2  |     7 |  71.43% |
| types                |    24  |     6  |    30 |  80.00% |
| v4                   |     4  |     1  |     5 |  80.00% |
| variable-declarator  |     0  |     1  |     1 |   0.00% |
| variance-annotations |     0  |     2  |     2 |   0.00% |
| vercel               |     1  |     0  |     1 | 100.00% |
|----------------------|--------|--------|-------|---------|
| total                |   390  |   148  |   538 |  72.49% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| early                |   263  |   405  |   668 |  39.37% |
| fail                 |   348  |   381  |   729 |  47.74% |
| pass                 |  1768  |   215  |  1983 |  89.16% |
| pass-explicit        |  1825  |   158  |  1983 |  92.03% |
|----------------------|--------|--------|-------|---------|
| total                |  4204  |  1159  |  5363 |  78.39% |
<!-- end:test262-results -->

## Why Not 100%?

We do not expect to reach 100% conformance because:

1. **Destack is TSX**: `.ds` files are more like `.tsx` than `.ts` or `.js`. Just like `.tsx` is not 100% compatible with `.ts`, `.ds` is not 100% compatible with `.ts` or `.js`.

2. **Destack is TypeScript++**: `.ds` files override some obscure TypeScript syntax patterns with more useful features (like tuples with `()` instead of sequence operators).

## Notes

- **Annex B**: The test262-parser-tests suite we use does not include Annex B tests (no legacy support).
- **Flow**: Flow is intentionally excluded because it's rarely used anymore. We support TypeScript only.
- **Strict**: Destack targets modern strict-mode JavaScript/TypeScript. Non-strict ("sloppy mode") behaviors like duplicate function declarations or `yield` as an identifier are not supported.
- **Proposals**: Some SWC `js/*` tests cover syntax proposals we don't support. These are intentionally excluded from conformance.