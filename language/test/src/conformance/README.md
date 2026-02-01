# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Skipped | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| babel    |   656  |    57  |     2  |   713 |  92.01% |
| biome    |   428  |   209  |     -  |   637 |  67.19% |
| swc      |   538  |     0  |     -  |   538 | 100.00% |
| test262  |  4230  |  1133  |     -  |  5363 |  78.87% |
|----------|--------|--------|---------|-------|---------|
| total    |  5852  |  1399  |      2  |  7251 |  80.71% |

Total Blended Pass Rate: **80.71%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| arrow-function       |    19  |     3  |       -  |    22 |  86.36% |
| assert-predicate     |     6  |     4  |       -  |    10 |  60.00% |
| assign               |     0  |     3  |       -  |     3 |   0.00% |
| async-call           |     0  |     1  |       -  |     1 |   0.00% |
| basic                |    37  |     0  |       -  |    37 | 100.00% |
| binary-expression    |     2  |     0  |       -  |     2 | 100.00% |
| cast                 |    38  |     4  |       -  |    42 |  90.48% |
| catch-clause         |     1  |     0  |       -  |     1 | 100.00% |
| class                |    94  |     3  |       -  |    97 |  96.91% |
| const                |     3  |     1  |       -  |     4 |  75.00% |
| declare              |    27  |     0  |       -  |    27 | 100.00% |
| decorators           |     1  |     1  |       -  |     2 |  50.00% |
| disallow-jsx-ambiguity |     1  |     2  |       -  |     3 |  33.33% |
| dts                  |     3  |     2  |       -  |     5 |  60.00% |
| enum                 |    13  |     0  |       -  |    13 | 100.00% |
| errors               |    26  |     2  |       -  |    28 |  92.86% |
| expect-plugin        |     0  |     3  |       -  |     3 |   0.00% |
| exponentiation       |     1  |     2  |       -  |     3 |  33.33% |
| export               |    10  |     3  |       -  |    13 |  76.92% |
| function             |    11  |     2  |       -  |    13 |  84.62% |
| html-entities        |     4  |     0  |       -  |     4 | 100.00% |
| import               |    18  |     3  |       -  |    21 |  85.71% |
| interface            |    47  |     0  |       -  |    47 | 100.00% |
| legacy-decorators    |     2  |     0  |       -  |     2 | 100.00% |
| module-namespace     |    13  |     3  |       2  |    16 |  81.25% |
| optional-chaining    |     1  |     0  |       -  |     1 | 100.00% |
| regression           |    24  |     1  |       -  |    25 |  96.00% |
| scope                |    62  |     4  |       -  |    66 |  93.94% |
| static-blocks        |    20  |     0  |       -  |    20 | 100.00% |
| tsx                  |     8  |     0  |       -  |     8 | 100.00% |
| type-alias           |     6  |     1  |       -  |     7 |  85.71% |
| type-arguments       |    30  |     3  |       -  |    33 |  90.91% |
| type-arguments-bit-shift-left-like |     9  |     0  |       -  |     9 | 100.00% |
| type-only-import-export-specifiers |    24  |     0  |       -  |    24 | 100.00% |
| types                |    90  |     5  |       -  |    95 |  94.74% |
| types-arrow-function |     3  |     0  |       -  |     3 | 100.00% |
| variable-declarator  |     2  |     1  |       -  |     3 |  66.67% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   656  |    57  |       2  |   713 |  92.01% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| error                |   165  |   137  |       -  |   302 |  54.64% |
| ok                   |   263  |    72  |       -  |   335 |  78.51% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   428  |   209  |       -  |   637 |  67.19% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| amaro-194            |     1  |     0  |       -  |     1 | 100.00% |
| arrow-function       |    15  |     0  |       -  |    15 | 100.00% |
| basic                |    63  |     0  |       -  |    63 | 100.00% |
| case1                |     1  |     0  |       -  |     1 | 100.00% |
| cast                 |    16  |     0  |       -  |    16 | 100.00% |
| class                |    39  |     0  |       -  |    39 | 100.00% |
| const                |     1  |     0  |       -  |     1 | 100.00% |
| custom               |    56  |     0  |       -  |    56 | 100.00% |
| declare              |     5  |     0  |       -  |     5 | 100.00% |
| decorators           |     2  |     0  |       -  |     2 | 100.00% |
| deno                 |     7  |     0  |       -  |     7 | 100.00% |
| enum                 |    11  |     0  |       -  |    11 | 100.00% |
| eof-issue            |     1  |     0  |       -  |     1 | 100.00% |
| errors               |    25  |     0  |       -  |    25 | 100.00% |
| es2019               |     1  |     0  |       -  |     1 | 100.00% |
| estree-compat        |     1  |     0  |       -  |     1 | 100.00% |
| export               |     5  |     0  |       -  |     5 | 100.00% |
| export-default-interface |     1  |     0  |       -  |     1 | 100.00% |
| function             |     6  |     0  |       -  |     6 | 100.00% |
| import               |     8  |     0  |       -  |     8 | 100.00% |
| import-assertions    |     7  |     0  |       -  |     7 | 100.00% |
| instantiation-expr   |     8  |     0  |       -  |     8 | 100.00% |
| interface            |    16  |     0  |       -  |    16 | 100.00% |
| issue                |   136  |     0  |       -  |   136 | 100.00% |
| meta-property        |     3  |     0  |       -  |     3 | 100.00% |
| module-namespace     |     9  |     0  |       -  |     9 | 100.00% |
| next                 |     2  |     0  |       -  |     2 | 100.00% |
| nullish-coalescing-operator |     7  |     0  |       -  |     7 | 100.00% |
| object               |     2  |     0  |       -  |     2 | 100.00% |
| optional-chaining    |    18  |     0  |       -  |    18 | 100.00% |
| regression           |     4  |     0  |       -  |     4 | 100.00% |
| stack-overflow       |     1  |     0  |       -  |     1 | 100.00% |
| stack-size           |     1  |     0  |       -  |     1 | 100.00% |
| stc                  |     2  |     0  |       -  |     2 | 100.00% |
| template-literal-type |     1  |     0  |       -  |     1 | 100.00% |
| top-level-await      |     1  |     0  |       -  |     1 | 100.00% |
| ts-import-type       |     1  |     0  |       -  |     1 | 100.00% |
| tsx                  |     3  |     0  |       -  |     3 | 100.00% |
| type-alias           |     5  |     0  |       -  |     5 | 100.00% |
| type-arguments       |     7  |     0  |       -  |     7 | 100.00% |
| types                |    30  |     0  |       -  |    30 | 100.00% |
| v4                   |     5  |     0  |       -  |     5 | 100.00% |
| variable-declarator  |     1  |     0  |       -  |     1 | 100.00% |
| variance-annotations |     2  |     0  |       -  |     2 | 100.00% |
| vercel               |     1  |     0  |       -  |     1 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   538  |     0  |       -  |   538 | 100.00% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| early                |   216  |   452  |       -  |   668 |  32.34% |
| fail                 |   341  |   388  |       -  |   729 |  46.78% |
| pass                 |  1809  |   174  |       -  |  1983 |  91.23% |
| pass-explicit        |  1864  |   119  |       -  |  1983 |  94.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  4230  |  1133  |       -  |  5363 |  78.87% |
<!-- end:test262-results -->

## Notes

We do not expect to reach 100% _general_ conformance because:

- **Modern TS modules only**: We target strict module semantics and do not support script mode.
- **JS restrictions**: `.js` and `.jsx` reject TS-only syntax and decorators, and JSX is only enabled in `.jsx`.
- **TypeScript++**: `.ds` files override obscure TypeScript patterns like the comma operator in favor of tuple syntax.
- **TSX ambiguity**: TSX generic arrows are ambiguous and require the standard `<T,>` workaround.
- **Annex B**: Legacy Annex B syntax is out of scope.
- **Flow**: Flow is intentionally excluded because we support TypeScript only.
- **Proposals**: Stage N proposals are out of scope unless explicitly documented.
