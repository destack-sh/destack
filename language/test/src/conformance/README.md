# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Status

The pass rate intentionally excludes the explicitly skipped tests.

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Skipped | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| babel    |   690  |     0  |    25  |   690 | 100.00% |
| biome    |   513  |   112  |    12  |   625 |  82.08% |
| swc      |   525  |     0  |    13  |   525 | 100.00% |
| test262  |  4410  |   953  |     -  |  5363 |  82.23% |
|----------|--------|--------|---------|-------|---------|
| total    |  6138  |  1065  |     50  |  7203 |  85.21% |

Total Blended Pass Rate: **85.21%**
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| arrow-function       |    21  |     0  |       1  |    21 | 100.00% |
| assert-predicate     |    10  |     0  |       -  |    10 | 100.00% |
| assign               |     3  |     0  |       -  |     3 | 100.00% |
| async-call           |     1  |     0  |       -  |     1 | 100.00% |
| basic                |    37  |     0  |       -  |    37 | 100.00% |
| binary-expression    |     2  |     0  |       -  |     2 | 100.00% |
| cast                 |    31  |     0  |      11  |    31 | 100.00% |
| catch-clause         |     1  |     0  |       -  |     1 | 100.00% |
| class                |    97  |     0  |       -  |    97 | 100.00% |
| const                |     4  |     0  |       -  |     4 | 100.00% |
| declare              |    27  |     0  |       -  |    27 | 100.00% |
| decorators           |     2  |     0  |       -  |     2 | 100.00% |
| disallow-jsx-ambiguity |     3  |     0  |       -  |     3 | 100.00% |
| dts                  |     5  |     0  |       -  |     5 | 100.00% |
| enum                 |    13  |     0  |       -  |    13 | 100.00% |
| errors               |    26  |     0  |       2  |    26 | 100.00% |
| expect-plugin        |     3  |     0  |       -  |     3 | 100.00% |
| exponentiation       |     1  |     0  |       2  |     1 | 100.00% |
| export               |    13  |     0  |       -  |    13 | 100.00% |
| function             |    13  |     0  |       -  |    13 | 100.00% |
| html-entities        |     4  |     0  |       -  |     4 | 100.00% |
| import               |    21  |     0  |       -  |    21 | 100.00% |
| interface            |    47  |     0  |       -  |    47 | 100.00% |
| legacy-decorators    |     2  |     0  |       -  |     2 | 100.00% |
| module-namespace     |    16  |     0  |       2  |    16 | 100.00% |
| optional-chaining    |     1  |     0  |       -  |     1 | 100.00% |
| regression           |    25  |     0  |       -  |    25 | 100.00% |
| scope                |    66  |     0  |       -  |    66 | 100.00% |
| static-blocks        |    20  |     0  |       -  |    20 | 100.00% |
| tsx                  |     8  |     0  |       -  |     8 | 100.00% |
| type-alias           |     7  |     0  |       -  |     7 | 100.00% |
| type-arguments       |    33  |     0  |       -  |    33 | 100.00% |
| type-arguments-bit-shift-left-like |     8  |     0  |       1  |     8 | 100.00% |
| type-only-import-export-specifiers |    24  |     0  |       -  |    24 | 100.00% |
| types                |    89  |     0  |       6  |    89 | 100.00% |
| types-arrow-function |     3  |     0  |       -  |     3 | 100.00% |
| variable-declarator  |     3  |     0  |       -  |     3 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   690  |     0  |      25  |   690 | 100.00% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| error                |   196  |   106  |       -  |   302 |  64.90% |
| ok                   |   317  |     6  |      12  |   323 |  98.14% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   513  |   112  |      12  |   625 |  82.08% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| amaro-194            |     1  |     0  |       -  |     1 | 100.00% |
| arrow-function       |    15  |     0  |       -  |    15 | 100.00% |
| basic                |    63  |     0  |       -  |    63 | 100.00% |
| case1                |     1  |     0  |       -  |     1 | 100.00% |
| cast                 |    11  |     0  |       5  |    11 | 100.00% |
| class                |    39  |     0  |       -  |    39 | 100.00% |
| const                |     1  |     0  |       -  |     1 | 100.00% |
| custom               |    52  |     0  |       4  |    52 | 100.00% |
| declare              |     5  |     0  |       -  |     5 | 100.00% |
| decorators           |     2  |     0  |       -  |     2 | 100.00% |
| deno                 |     6  |     0  |       1  |     6 | 100.00% |
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
| issue                |   135  |     0  |       1  |   135 | 100.00% |
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
| variance-annotations |     0  |     0  |       2  |     0 | 100.00% |
| vercel               |     1  |     0  |       -  |     1 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   525  |     0  |      13  |   525 | 100.00% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Skipped | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| early                |   338  |   330  |       -  |   668 |  50.60% |
| fail                 |   380  |   349  |       -  |   729 |  52.13% |
| pass                 |  1829  |   154  |       -  |  1983 |  92.23% |
| pass-explicit        |  1863  |   120  |       -  |  1983 |  93.95% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  4410  |   953  |       -  |  5363 |  82.23% |
<!-- end:test262-results -->

## Notes

We do not expect to reach 100% _general_ conformance because:

- **Modern TS modules only**: We target strict module semantics and do not support script mode.
- **JS restrictions**: `.js` and `.jsx` reject TS-only syntax and decorators, and JSX is only enabled in `.jsx`.
- **JSDoc typing**: JSDoc-based typing and `@ts-check` semantics are out of scope.
- **TypeScript++**: `.ds` files override obscure TypeScript patterns like the comma operator in favor of tuple syntax.
- **TSX ambiguity**: Some suites disallow ambiguous JSX-like syntax, so generic arrows may require `<T,>` or `extends` disambiguators there.
- **Annex B**: Legacy Annex B syntax is out of scope.
- **Flow**: Flow is intentionally excluded because we support TypeScript only.
- **Proposals**: Stage N proposals are out of scope unless explicitly documented.
