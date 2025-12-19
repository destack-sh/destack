# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Skipped | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| babel    |   479  |   229  |     7  |   708 |  67.66% |
| biome    |   410  |   227  |     -  |   637 |  64.36% |
| swc      |   392  |   146  |     -  |   538 |  72.86% |
| test262  |  4305  |  1058  |     -  |  5363 |  80.27% |
|----------|--------|--------|---------|-------|---------|
| total    |  5586  |  1660  |      7  |  7246 |  77.09% |

Total Blended Pass Rate: **77.09%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| arrow-function       |    12  |    10  |    22 |  54.55% |
| assert-predicate     |     7  |     3  |    10 |  70.00% |
| assign               |     1  |     2  |     3 |  33.33% |
| async-call           |     0  |     1  |     1 |   0.00% |
| basic                |    29  |     8  |    37 |  78.38% |
| binary-expression    |     2  |     0  |     2 | 100.00% |
| cast                 |    30  |     7  |    37 |  81.08% |
| catch-clause         |     1  |     0  |     1 | 100.00% |
| class                |    57  |    40  |    97 |  58.76% |
| const                |     1  |     3  |     4 |  25.00% |
| declare              |    15  |    12  |    27 |  55.56% |
| decorators           |     1  |     1  |     2 |  50.00% |
| disallow-jsx-ambiguity |     3  |     0  |     3 | 100.00% |
| dts                  |     3  |     2  |     5 |  60.00% |
| enum                 |    13  |     0  |    13 | 100.00% |
| errors               |    19  |     9  |    28 |  67.86% |
| expect-plugin        |     0  |     3  |     3 |   0.00% |
| exponentiation       |     1  |     2  |     3 |  33.33% |
| export               |    10  |     3  |    13 |  76.92% |
| function             |     9  |     4  |    13 |  69.23% |
| html-entities        |     3  |     1  |     4 |  75.00% |
| import               |     8  |    13  |    21 |  38.10% |
| interface            |    33  |    14  |    47 |  70.21% |
| legacy-decorators    |     1  |     1  |     2 |  50.00% |
| module-namespace     |    13  |     3  |    16 |  81.25% |
| optional-chaining    |     1  |     0  |     1 | 100.00% |
| regression           |    22  |     3  |    25 |  88.00% |
| scope                |    43  |    23  |    66 |  65.15% |
| static-blocks        |    19  |     1  |    20 |  95.00% |
| tsx                  |     5  |     3  |     8 |  62.50% |
| type-alias           |     5  |     2  |     7 |  71.43% |
| type-arguments       |    27  |     6  |    33 |  81.82% |
| type-arguments-bit-shift-left-like |     3  |     6  |     9 |  33.33% |
| type-only-import-export-specifiers |    15  |     9  |    24 |  62.50% |
| types                |    63  |    32  |    95 |  66.32% |
| types-arrow-function |     3  |     0  |     3 | 100.00% |
| variable-declarator  |     1  |     2  |     3 |  33.33% |
|----------------------|--------|--------|-------|---------|
| total                |   479  |   229  |   708 |  67.66% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| error                |   199  |   103  |   302 |  65.89% |
| ok                   |   211  |   124  |   335 |  62.99% |
|----------------------|--------|--------|-------|---------|
| total                |   410  |   227  |   637 |  64.36% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| amaro-194            |     0  |     1  |     1 |   0.00% |
| arrow-function       |    10  |     5  |    15 |  66.67% |
| basic                |    45  |    18  |    63 |  71.43% |
| case1                |     1  |     0  |     1 | 100.00% |
| cast                 |    11  |     5  |    16 |  68.75% |
| class                |    18  |    21  |    39 |  46.15% |
| const                |     1  |     0  |     1 | 100.00% |
| custom               |    40  |    16  |    56 |  71.43% |
| declare              |     5  |     0  |     5 | 100.00% |
| decorators           |     0  |     2  |     2 |   0.00% |
| deno                 |     6  |     1  |     7 |  85.71% |
| enum                 |    11  |     0  |    11 | 100.00% |
| eof-issue            |     1  |     0  |     1 | 100.00% |
| errors               |    15  |    10  |    25 |  60.00% |
| es2019               |     0  |     1  |     1 |   0.00% |
| estree-compat        |     1  |     0  |     1 | 100.00% |
| export               |     4  |     1  |     5 |  80.00% |
| export-default-interface |     1  |     0  |     1 | 100.00% |
| function             |     5  |     1  |     6 |  83.33% |
| import               |     6  |     2  |     8 |  75.00% |
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
| total                |   392  |   146  |   538 |  72.86% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| early                |   375  |   293  |   668 |  56.14% |
| fail                 |   355  |   374  |   729 |  48.70% |
| pass                 |  1759  |   224  |  1983 |  88.70% |
| pass-explicit        |  1816  |   167  |  1983 |  91.58% |
|----------------------|--------|--------|-------|---------|
| total                |  4305  |  1058  |  5363 |  80.27% |
<!-- end:test262-results -->

## Notes

We do not expect to reach 100% conformance because:

- **TSX**: `.ds` files are more like `.tsx` than `.ts` or `.js`. Just like `.tsx` is not 100% compatible with `.ts`, `.ds` is not 100% compatible with `.ts` or `.js`. (We still strive for 100% compatibility *within* `.js` and `.ts` files.)
- **TypeScript++**: `.ds` files override some obscure TypeScript syntax patterns with more useful features (like tuples with `()` instead of sequence operators).
- **Annex B**: Our test262 suite does not include Annex B tests. We do not support legacy syntax.
- **Flow**: Flow is intentionally excluded because it's rarely used anymore. We target modern JS/TS only.
- **Strict**: Non-strict ("sloppy mode") behaviors are not supported. We target strict-mode JS/TS. 
- **Proposals**: Some SWC `js/*` tests cover not-yet-standard syntax proposals. We do not aim to support most of these.