# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Skipped | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| babel    |   482  |   228  |     5  |   710 |  67.89% |
| biome    |   412  |   225  |     -  |   637 |  64.68% |
| swc      |   401  |   137  |     -  |   538 |  74.54% |
| test262  |  4360  |  1003  |     -  |  5363 |  81.30% |
|----------|--------|--------|---------|-------|---------|
| total    |  5655  |  1593  |      5  |  7248 |  78.02% |

Total Blended Pass Rate: **78.02%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| arrow-function       |    12  |    10  |    22 |  54.55% |
| assert-predicate     |     7  |     3  |    10 |  70.00% |
| assign               |     1  |     2  |     3 |  33.33% |
| async-call           |     0  |     1  |     1 |   0.00% |
| basic                |    28  |     9  |    37 |  75.68% |
| binary-expression    |     2  |     0  |     2 | 100.00% |
| cast                 |    30  |     7  |    37 |  81.08% |
| catch-clause         |     1  |     0  |     1 | 100.00% |
| class                |    56  |    41  |    97 |  57.73% |
| const                |     1  |     3  |     4 |  25.00% |
| declare              |    13  |    14  |    27 |  48.15% |
| decorators           |     1  |     1  |     2 |  50.00% |
| disallow-jsx-ambiguity |     3  |     0  |     3 | 100.00% |
| dts                  |     3  |     2  |     5 |  60.00% |
| enum                 |    13  |     0  |    13 | 100.00% |
| errors               |    19  |     9  |    28 |  67.86% |
| expect-plugin        |     0  |     3  |     3 |   0.00% |
| exponentiation       |     1  |     2  |     3 |  33.33% |
| export               |    11  |     2  |    13 |  84.62% |
| function             |     9  |     4  |    13 |  69.23% |
| html-entities        |     3  |     1  |     4 |  75.00% |
| import               |    13  |     8  |    21 |  61.90% |
| interface            |    33  |    14  |    47 |  70.21% |
| legacy-decorators    |     1  |     1  |     2 |  50.00% |
| module-namespace     |    15  |     3  |    18 |  83.33% |
| optional-chaining    |     1  |     0  |     1 | 100.00% |
| regression           |    21  |     4  |    25 |  84.00% |
| scope                |    46  |    20  |    66 |  69.70% |
| static-blocks        |    18  |     2  |    20 |  90.00% |
| tsx                  |     5  |     3  |     8 |  62.50% |
| type-alias           |     5  |     2  |     7 |  71.43% |
| type-arguments       |    27  |     6  |    33 |  81.82% |
| type-arguments-bit-shift-left-like |     3  |     6  |     9 |  33.33% |
| type-only-import-export-specifiers |    14  |    10  |    24 |  58.33% |
| types                |    62  |    33  |    95 |  65.26% |
| types-arrow-function |     3  |     0  |     3 | 100.00% |
| variable-declarator  |     1  |     2  |     3 |  33.33% |
|----------------------|--------|--------|-------|---------|
| total                |   482  |   228  |   710 |  67.89% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| error                |   195  |   107  |   302 |  64.57% |
| ok                   |   217  |   118  |   335 |  64.78% |
|----------------------|--------|--------|-------|---------|
| total                |   412  |   225  |   637 |  64.68% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| amaro-194            |     0  |     1  |     1 |   0.00% |
| arrow-function       |     9  |     6  |    15 |  60.00% |
| basic                |    46  |    17  |    63 |  73.02% |
| case1                |     1  |     0  |     1 | 100.00% |
| cast                 |    11  |     5  |    16 |  68.75% |
| class                |    18  |    21  |    39 |  46.15% |
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
| import               |     7  |     1  |     8 |  87.50% |
| import-assertions    |     7  |     0  |     7 | 100.00% |
| instantiation-expr   |     6  |     2  |     8 |  75.00% |
| interface            |    16  |     0  |    16 | 100.00% |
| issue                |    99  |    37  |   136 |  72.79% |
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
| total                |   401  |   137  |   538 |  74.54% |
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

- **Annex B**: Our test262 suite does not include Annex B tests. We do not support legacy syntax.
- **Flow**: Flow is intentionally excluded because it's rarely used anymore. We target modern JS/TS only.
- **Strict**: Non-strict ("sloppy mode") behaviors are not supported. We target strict-mode JS/TS. 
- **Proposals**: Some SWC `js/*` tests cover not-yet-standard syntax proposals. We do not aim to support most of these.