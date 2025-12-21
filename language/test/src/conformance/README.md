# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Skipped | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| babel    |   342  |   366  |     7  |   708 |  48.31% |
| biome    |   329  |   308  |     -  |   637 |  51.65% |
| swc      |   173  |   365  |     -  |   538 |  32.16% |
| test262  |  2775  |  2588  |     -  |  5363 |  51.74% |
|----------|--------|--------|---------|-------|---------|
| total    |  3619  |  3627  |      7  |  7246 |  49.94% |

Total Blended Pass Rate: **49.94%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| arrow-function       |    10  |    12  |    22 |  45.45% |
| assert-predicate     |     1  |     9  |    10 |  10.00% |
| assign               |     3  |     0  |     3 | 100.00% |
| async-call           |     0  |     1  |     1 |   0.00% |
| basic                |     3  |    34  |    37 |   8.11% |
| binary-expression    |     1  |     1  |     2 |  50.00% |
| cast                 |    12  |    25  |    37 |  32.43% |
| catch-clause         |     1  |     0  |     1 | 100.00% |
| class                |    45  |    52  |    97 |  46.39% |
| const                |     2  |     2  |     4 |  50.00% |
| declare              |    18  |     9  |    27 |  66.67% |
| decorators           |     1  |     1  |     2 |  50.00% |
| disallow-jsx-ambiguity |     3  |     0  |     3 | 100.00% |
| dts                  |     3  |     2  |     5 |  60.00% |
| enum                 |    12  |     1  |    13 |  92.31% |
| errors               |    23  |     5  |    28 |  82.14% |
| expect-plugin        |     0  |     3  |     3 |   0.00% |
| exponentiation       |     1  |     2  |     3 |  33.33% |
| export               |     6  |     7  |    13 |  46.15% |
| function             |     4  |     9  |    13 |  30.77% |
| html-entities        |     0  |     4  |     4 |   0.00% |
| import               |     2  |    19  |    21 |   9.52% |
| interface            |    31  |    16  |    47 |  65.96% |
| legacy-decorators    |     0  |     2  |     2 |   0.00% |
| module-namespace     |     7  |     9  |    16 |  43.75% |
| optional-chaining    |     0  |     1  |     1 |   0.00% |
| regression           |     5  |    20  |    25 |  20.00% |
| scope                |    41  |    25  |    66 |  62.12% |
| static-blocks        |    18  |     2  |    20 |  90.00% |
| tsx                  |     3  |     5  |     8 |  37.50% |
| type-alias           |     5  |     2  |     7 |  71.43% |
| type-arguments       |    13  |    20  |    33 |  39.39% |
| type-arguments-bit-shift-left-like |     0  |     9  |     9 |   0.00% |
| type-only-import-export-specifiers |    16  |     8  |    24 |  66.67% |
| types                |    48  |    47  |    95 |  50.53% |
| types-arrow-function |     3  |     0  |     3 | 100.00% |
| variable-declarator  |     1  |     2  |     3 |  33.33% |
|----------------------|--------|--------|-------|---------|
| total                |   342  |   366  |   708 |  48.31% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| error                |   237  |    65  |   302 |  78.48% |
| ok                   |    92  |   243  |   335 |  27.46% |
|----------------------|--------|--------|-------|---------|
| total                |   329  |   308  |   637 |  51.65% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| amaro-194            |     0  |     1  |     1 |   0.00% |
| arrow-function       |     7  |     8  |    15 |  46.67% |
| basic                |     8  |    55  |    63 |  12.70% |
| case1                |     1  |     0  |     1 | 100.00% |
| cast                 |     0  |    16  |    16 |   0.00% |
| class                |    10  |    29  |    39 |  25.64% |
| const                |     1  |     0  |     1 | 100.00% |
| custom               |    22  |    34  |    56 |  39.29% |
| declare              |     5  |     0  |     5 | 100.00% |
| decorators           |     0  |     2  |     2 |   0.00% |
| deno                 |     2  |     5  |     7 |  28.57% |
| enum                 |    11  |     0  |    11 | 100.00% |
| eof-issue            |     0  |     1  |     1 |   0.00% |
| errors               |    21  |     4  |    25 |  84.00% |
| es2019               |     0  |     1  |     1 |   0.00% |
| estree-compat        |     0  |     1  |     1 |   0.00% |
| export               |     1  |     4  |     5 |  20.00% |
| export-default-interface |     1  |     0  |     1 | 100.00% |
| function             |     1  |     5  |     6 |  16.67% |
| import               |     0  |     8  |     8 |   0.00% |
| import-assertions    |     0  |     7  |     7 |   0.00% |
| instantiation-expr   |     0  |     8  |     8 |   0.00% |
| interface            |    11  |     5  |    16 |  68.75% |
| issue                |    41  |    95  |   136 |  30.15% |
| meta-property        |     0  |     3  |     3 |   0.00% |
| module-namespace     |     4  |     5  |     9 |  44.44% |
| next                 |     0  |     2  |     2 |   0.00% |
| nullish-coalescing-operator |     0  |     7  |     7 |   0.00% |
| object               |     1  |     1  |     2 |  50.00% |
| optional-chaining    |     0  |    18  |    18 |   0.00% |
| regression           |     2  |     2  |     4 |  50.00% |
| stack-overflow       |     0  |     1  |     1 |   0.00% |
| stack-size           |     0  |     1  |     1 |   0.00% |
| stc                  |     1  |     1  |     2 |  50.00% |
| template-literal-type |     1  |     0  |     1 | 100.00% |
| top-level-await      |     0  |     1  |     1 |   0.00% |
| ts-import-type       |     0  |     1  |     1 |   0.00% |
| tsx                  |     0  |     3  |     3 |   0.00% |
| type-alias           |     4  |     1  |     5 |  80.00% |
| type-arguments       |     1  |     6  |     7 |  14.29% |
| types                |    13  |    17  |    30 |  43.33% |
| v4                   |     3  |     2  |     5 |  60.00% |
| variable-declarator  |     0  |     1  |     1 |   0.00% |
| variance-annotations |     0  |     2  |     2 |   0.00% |
| vercel               |     0  |     1  |     1 |   0.00% |
|----------------------|--------|--------|-------|---------|
| total                |   173  |   365  |   538 |  32.16% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| early                |   506  |   162  |   668 |  75.75% |
| fail                 |   473  |   256  |   729 |  64.88% |
| pass                 |   878  |  1105  |  1983 |  44.28% |
| pass-explicit        |   918  |  1065  |  1983 |  46.29% |
|----------------------|--------|--------|-------|---------|
| total                |  2775  |  2588  |  5363 |  51.74% |
<!-- end:test262-results -->

## Notes

We do not expect to reach 100% conformance because:

- **TSX**: `.ds` files are more like `.tsx` than `.ts` or `.js`. Just like `.tsx` is not 100% compatible with `.ts`, `.ds` is not 100% compatible with `.ts` or `.js`. (We still strive for 100% compatibility *within* `.js` and `.ts` files.)
- **TypeScript++**: `.ds` files override some obscure TypeScript syntax patterns with more useful features (like tuples with `()` instead of sequence operators).
- **Annex B**: Our test262 suite does not include Annex B tests. We do not support legacy syntax.
- **Flow**: Flow is intentionally excluded because it's rarely used anymore. We target modern JS/TS only.
- **Strict**: Non-strict ("sloppy mode") behaviors are not supported. We target strict-mode JS/TS. 
- **Proposals**: Some SWC `js/*` tests cover not-yet-standard syntax proposals. We do not aim to support most of these.