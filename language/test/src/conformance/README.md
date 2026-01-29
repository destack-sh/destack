# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Skipped | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| babel    |   520  |   188  |     7  |   708 |  73.45% |
| biome    |   428  |   209  |     -  |   637 |  67.19% |
| swc      |   447  |    91  |     -  |   538 |  83.09% |
| test262  |  4330  |  1033  |     -  |  5363 |  80.74% |
|----------|--------|--------|---------|-------|---------|
| total    |  5725  |  1521  |      7  |  7246 |  79.01% |

Total Blended Pass Rate: **79.01%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| arrow-function       |    14  |     8  |    22 |  63.64% |
| assert-predicate     |     6  |     4  |    10 |  60.00% |
| assign               |     0  |     3  |     3 |   0.00% |
| async-call           |     0  |     1  |     1 |   0.00% |
| basic                |    29  |     8  |    37 |  78.38% |
| binary-expression    |     2  |     0  |     2 | 100.00% |
| cast                 |    32  |     5  |    37 |  86.49% |
| catch-clause         |     1  |     0  |     1 | 100.00% |
| class                |    56  |    41  |    97 |  57.73% |
| const                |     2  |     2  |     4 |  50.00% |
| declare              |    20  |     7  |    27 |  74.07% |
| decorators           |     2  |     0  |     2 | 100.00% |
| disallow-jsx-ambiguity |     1  |     2  |     3 |  33.33% |
| dts                  |     3  |     2  |     5 |  60.00% |
| enum                 |    13  |     0  |    13 | 100.00% |
| errors               |    19  |     9  |    28 |  67.86% |
| expect-plugin        |     0  |     3  |     3 |   0.00% |
| exponentiation       |     1  |     2  |     3 |  33.33% |
| export               |     8  |     5  |    13 |  61.54% |
| function             |    11  |     2  |    13 |  84.62% |
| html-entities        |     3  |     1  |     4 |  75.00% |
| import               |    16  |     5  |    21 |  76.19% |
| interface            |    29  |    18  |    47 |  61.70% |
| legacy-decorators    |     2  |     0  |     2 | 100.00% |
| module-namespace     |     9  |     7  |    16 |  56.25% |
| optional-chaining    |     1  |     0  |     1 | 100.00% |
| regression           |    24  |     1  |    25 |  96.00% |
| scope                |    61  |     5  |    66 |  92.42% |
| static-blocks        |    16  |     4  |    20 |  80.00% |
| tsx                  |     5  |     3  |     8 |  62.50% |
| type-alias           |     6  |     1  |     7 |  85.71% |
| type-arguments       |    29  |     4  |    33 |  87.88% |
| type-arguments-bit-shift-left-like |     6  |     3  |     9 |  66.67% |
| type-only-import-export-specifiers |    14  |    10  |    24 |  58.33% |
| types                |    74  |    21  |    95 |  77.89% |
| types-arrow-function |     3  |     0  |     3 | 100.00% |
| variable-declarator  |     2  |     1  |     3 |  66.67% |
|----------------------|--------|--------|-------|---------|
| total                |   520  |   188  |   708 |  73.45% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| error                |   167  |   135  |   302 |  55.30% |
| ok                   |   261  |    74  |   335 |  77.91% |
|----------------------|--------|--------|-------|---------|
| total                |   428  |   209  |   637 |  67.19% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| amaro-194            |     1  |     0  |     1 | 100.00% |
| arrow-function       |    14  |     1  |    15 |  93.33% |
| basic                |    49  |    14  |    63 |  77.78% |
| case1                |     1  |     0  |     1 | 100.00% |
| cast                 |    16  |     0  |    16 | 100.00% |
| class                |    25  |    14  |    39 |  64.10% |
| const                |     1  |     0  |     1 | 100.00% |
| custom               |    45  |    11  |    56 |  80.36% |
| declare              |     5  |     0  |     5 | 100.00% |
| decorators           |     2  |     0  |     2 | 100.00% |
| deno                 |     6  |     1  |     7 |  85.71% |
| enum                 |    11  |     0  |    11 | 100.00% |
| eof-issue            |     1  |     0  |     1 | 100.00% |
| errors               |    15  |    10  |    25 |  60.00% |
| es2019               |     1  |     0  |     1 | 100.00% |
| estree-compat        |     1  |     0  |     1 | 100.00% |
| export               |     3  |     2  |     5 |  60.00% |
| export-default-interface |     1  |     0  |     1 | 100.00% |
| function             |     6  |     0  |     6 | 100.00% |
| import               |     7  |     1  |     8 |  87.50% |
| import-assertions    |     5  |     2  |     7 |  71.43% |
| instantiation-expr   |     7  |     1  |     8 |  87.50% |
| interface            |    16  |     0  |    16 | 100.00% |
| issue                |   115  |    21  |   136 |  84.56% |
| meta-property        |     3  |     0  |     3 | 100.00% |
| module-namespace     |     6  |     3  |     9 |  66.67% |
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
| ts-import-type       |     0  |     1  |     1 |   0.00% |
| tsx                  |     2  |     1  |     3 |  66.67% |
| type-alias           |     5  |     0  |     5 | 100.00% |
| type-arguments       |     6  |     1  |     7 |  85.71% |
| types                |    29  |     1  |    30 |  96.67% |
| v4                   |     5  |     0  |     5 | 100.00% |
| variable-declarator  |     1  |     0  |     1 | 100.00% |
| variance-annotations |     0  |     2  |     2 |   0.00% |
| vercel               |     1  |     0  |     1 | 100.00% |
|----------------------|--------|--------|-------|---------|
| total                |   447  |    91  |   538 |  83.09% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Total |  Rate   |
|:---------------------|-------:|-------:|------:|--------:|
| early                |   319  |   349  |   668 |  47.75% |
| fail                 |   338  |   391  |   729 |  46.36% |
| pass                 |  1809  |   174  |  1983 |  91.23% |
| pass-explicit        |  1864  |   119  |  1983 |  94.00% |
|----------------------|--------|--------|-------|---------|
| total                |  4330  |  1033  |  5363 |  80.74% |
<!-- end:test262-results -->

## Notes

We do not expect to reach 100% conformance because:

- **Modern TS modules only**: We target strict module semantics and do not support script mode.
- **JS restrictions**: `.js` and `.jsx` reject TS-only syntax and decorators, and JSX is only enabled in `.jsx`.
- **TypeScript++**: `.ds` files override obscure TypeScript patterns like the comma operator in favor of tuple syntax.
- **TSX ambiguity**: TSX generic arrows are ambiguous and require the standard `<T,>` workaround.
- **Annex B**: Legacy Annex B syntax is out of scope.
- **Flow**: Flow is intentionally excluded because we support TypeScript only.
- **Proposals**: Stage N proposals are out of scope unless explicitly documented.
