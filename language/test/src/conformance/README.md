# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Total |  Rate   |
|:---------|-------:|-------:|------:|--------:|
| babel    |   455  |   265  |   720 |  63.19% |
| biome    |   403  |   234  |   637 |  63.27% |
| swc      |   391  |   147  |   538 |  72.68% |
| test262  |  4360  |  1003  |  5363 |  81.30% |
|----------|--------|--------|-------|---------|
| total    |  5609  |  1649  |  7258 |  77.28% |

Total Blended Pass Rate: **77.28%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| arrow-function       |    12  |    10  |    22 |  54.55% |
| assert-predicate     |     7  |     3  |    10 |  70.00% |
| assign               |     1  |     2  |     3 |  33.33% |
| async-call           |     0  |     1  |     1 |   0.00% |
| basic                |    28  |    10  |    38 |  73.68% |
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
| function             |     7  |     6  |    13 |  53.85% |
| html-entities        |     3  |     1  |     4 |  75.00% |
| import               |     9  |    14  |    23 |  39.13% |
| interface            |    30  |    17  |    47 |  63.83% |
| legacy-decorators    |     1  |     1  |     2 |  50.00% |
| module-namespace     |    15  |     3  |    18 |  83.33% |
| optional-chaining    |     1  |     0  |     1 | 100.00% |
| regression           |    22  |     3  |    25 |  88.00% |
| scope                |    46  |    20  |    66 |  69.70% |
| static-blocks        |    14  |     6  |    20 |  70.00% |
| tsx                  |     5  |     3  |     8 |  62.50% |
| type-alias           |     5  |     2  |     7 |  71.43% |
| type-arguments       |    22  |    11  |    33 |  66.67% |
| type-arguments-bit-shift-left-like |     3  |     6  |     9 |  33.33% |
| type-only-import-export-specifiers |    14  |    10  |    24 |  58.33% |
| types                |    63  |    32  |    95 |  66.32% |
| types-arrow-function |     3  |     0  |     3 | 100.00% |
| variable-declarator  |     1  |     2  |     3 |  33.33% |
|----------------------|--------|--------|-------|---------|
| total                |   455  |   265  |   720 |  63.19% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| error                |   188  |   114  |   302 |  62.25% |
| ok                   |   215  |   120  |   335 |  64.18% |
|----------------------|--------|--------|-------|---------|
| total                |   403  |   234  |   637 |  63.27% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| amaro-194            |     0  |     1  |     1 |   0.00% |
| arrow-function       |     8  |     7  |    15 |  53.33% |
| basic                |    46  |    17  |    63 |  73.02% |
| case1                |     1  |     0  |     1 | 100.00% |
| cast                 |    11  |     5  |    16 |  68.75% |
| class                |    19  |    20  |    39 |  48.72% |
| const                |     1  |     0  |     1 | 100.00% |
| custom               |    43  |    13  |    56 |  76.79% |
| declare              |     5  |     0  |     5 | 100.00% |
| decorators           |     0  |     2  |     2 |   0.00% |
| deno                 |     6  |     1  |     7 |  85.71% |
| enum                 |    11  |     0  |    11 | 100.00% |
| eof-issue            |     1  |     0  |     1 | 100.00% |
| errors               |    15  |    10  |    25 |  60.00% |
| es2019               |     0  |     1  |     1 |   0.00% |
| estree-compat        |     1  |     0  |     1 | 100.00% |
| export               |     5  |     0  |     5 | 100.00% |
| export-default-interface |     1  |     0  |     1 | 100.00% |
| function             |     5  |     1  |     6 |  83.33% |
| import               |     1  |     7  |     8 |  12.50% |
| import-assertions    |     7  |     0  |     7 | 100.00% |
| instantiation-expr   |     6  |     2  |     8 |  75.00% |
| interface            |    16  |     0  |    16 | 100.00% |
| issue                |    95  |    41  |   136 |  69.85% |
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
| types                |    23  |     7  |    30 |  76.67% |
| v4                   |     4  |     1  |     5 |  80.00% |
| variable-declarator  |     0  |     1  |     1 |   0.00% |
| variance-annotations |     0  |     2  |     2 |   0.00% |
| vercel               |     1  |     0  |     1 | 100.00% |
|----------------------|--------|--------|-------|---------|
| total                |   391  |   147  |   538 |  72.68% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| early                |   363  |   305  |   668 |  54.34% |
| fail                 |   346  |   383  |   729 |  47.46% |
| pass                 |  1798  |   185  |  1983 |  90.67% |
| pass-explicit        |  1853  |   130  |  1983 |  93.44% |
|----------------------|--------|--------|-------|---------|
| total                |  4360  |  1003  |  5363 |  81.30% |
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