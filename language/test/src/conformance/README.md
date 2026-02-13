# Conformance Tests

Conformance tests check that the Destack parser conforms both to the ECMAScript specification and various other established "real-world" test suites.

## Objectives

The point of testing conformance is to ensure that modern TS-first libraries can just work straight in Destack without major - or ideally _any_ - changes.
However, we do not expect or need to reach 100% _general_ conformance across all suites because:

- **Modern (TS) ESM**: We target strict module semantics and do not support script mode and some legacy or otherwise unsupported JS/TS syntax.
- **CommonJS interop**: Top-level `module.exports` assignments are supported for default import compatibility without enabling script mode semantics.
- **Annex B**: Legacy Annex B syntax is out of scope.
- **JS-only in JS**: `.js` and `.jsx` reject TS-only syntax and decorators, and JSX is only enabled in `.jsx`.
- **TypeScript++**: `.ds` files override obscure TypeScript patterns like the comma operator in favor of tuple syntax.
- **TSX ambiguity**: Some suites disallow ambiguous JSX-like syntax, so generic arrows may require `<T,>` or `extends` disambiguators there.
- **Flow**: Flow is intentionally excluded (though it usually overlaps with TS anyway).
- **JSDoc typing**: JSDoc-based typing and `@ts-check` semantics are out of scope.
- **Proposals**: Stage N proposals are out of scope unless explicitly documented (we do support `using`).
- **Conflicting tests**: Some test suites are mutually conflicting (i.e. you can't make them both pass without suite-specific hacks), so in general we align with the "modern standard"

## Status

The pass rate intentionally excludes the explicitly ignored tests.
As described above, we exclude a small subset of legacy, non-standard and mutually conflicting test expectations.
(See the individual *-ignored.txt files for details.)
When conformance fixtures are missing, the runner attempts to fetch them automatically with the suite fetch scripts.
If auto-fetch cannot satisfy a suite, the run fails with an explicit error instead of skipping.

<!-- (results are automatically updated by the conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------|-------:|-------:|--------:|------:|--------:|-----------:|
| babel    |   702  |     1  |    12  |   703 |  99.86% |  98.18% |
| biome    |   614  |     0  |    23  |   614 | 100.00% |  96.39% |
| swc      |   535  |     1  |     2  |   536 |  99.81% |  99.44% |
| test262  |  5174  |     0  |   189  |  5174 | 100.00% |  96.48% |
|----------|--------|--------|---------|-------|---------|------------|
| total    |  7025  |     2  |    226  |  7027 |  99.97% |     96.86% |

Total Blended Pass Rate: **99.97%** (96.86% incl. ignored)
<!-- end:summary-results -->

### babel
<!-- begin:babel-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| arrow-function       |    20  |     1  |       1  |    21 |  95.24% |     90.91% |
| assert-predicate     |    10  |     0  |       -  |    10 | 100.00% |    100.00% |
| assign               |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| async-call           |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| basic                |    37  |     0  |       -  |    37 | 100.00% |    100.00% |
| binary-expression    |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| cast                 |    42  |     0  |       -  |    42 | 100.00% |    100.00% |
| catch-clause         |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| class                |    97  |     0  |       -  |    97 | 100.00% |    100.00% |
| const                |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| declare              |    27  |     0  |       -  |    27 | 100.00% |    100.00% |
| decorators           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| disallow-jsx-ambiguity |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| dts                  |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| enum                 |    13  |     0  |       -  |    13 | 100.00% |    100.00% |
| errors               |    26  |     0  |       2  |    26 | 100.00% |     92.86% |
| expect-plugin        |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| exponentiation       |     2  |     0  |       1  |     2 | 100.00% |     66.67% |
| export               |    13  |     0  |       -  |    13 | 100.00% |    100.00% |
| function             |    13  |     0  |       -  |    13 | 100.00% |    100.00% |
| html-entities        |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| import               |    21  |     0  |       -  |    21 | 100.00% |    100.00% |
| interface            |    47  |     0  |       -  |    47 | 100.00% |    100.00% |
| legacy-decorators    |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| module-namespace     |    16  |     0  |       2  |    16 | 100.00% |     88.89% |
| optional-chaining    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| regression           |    25  |     0  |       -  |    25 | 100.00% |    100.00% |
| scope                |    66  |     0  |       -  |    66 | 100.00% |    100.00% |
| static-blocks        |    20  |     0  |       -  |    20 | 100.00% |    100.00% |
| tsx                  |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| type-alias           |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| type-arguments       |    33  |     0  |       -  |    33 | 100.00% |    100.00% |
| type-arguments-bit-shift-left-like |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| type-only-import-export-specifiers |    24  |     0  |       -  |    24 | 100.00% |    100.00% |
| types                |    89  |     0  |       6  |    89 | 100.00% |     93.68% |
| types-arrow-function |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| variable-declarator  |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |   702  |     1  |      12  |   703 |  99.86% |     98.18% |
<!-- end:babel-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| error                |   285  |     0  |      17  |   285 | 100.00% |     94.37% |
| ok                   |   329  |     0  |       6  |   329 | 100.00% |     98.21% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |   614  |     0  |      23  |   614 | 100.00% |     96.39% |
<!-- end:biome-results -->

### swc
<!-- begin:swc-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| amaro-194            |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| arrow-function       |    14  |     1  |       -  |    15 |  93.33% |     93.33% |
| basic                |    63  |     0  |       -  |    63 | 100.00% |    100.00% |
| case1                |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| cast                 |    16  |     0  |       -  |    16 | 100.00% |    100.00% |
| class                |    39  |     0  |       -  |    39 | 100.00% |    100.00% |
| const                |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| custom               |    56  |     0  |       -  |    56 | 100.00% |    100.00% |
| declare              |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| decorators           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| deno                 |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| enum                 |    11  |     0  |       -  |    11 | 100.00% |    100.00% |
| eof-issue            |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| errors               |    25  |     0  |       -  |    25 | 100.00% |    100.00% |
| es2019               |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| estree-compat        |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| export               |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| export-default-interface |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| function             |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| import               |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| import-assertions    |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| instantiation-expr   |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| interface            |    16  |     0  |       -  |    16 | 100.00% |    100.00% |
| issue                |   136  |     0  |       -  |   136 | 100.00% |    100.00% |
| meta-property        |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| module-namespace     |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| next                 |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| nullish-coalescing-operator |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| object               |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| optional-chaining    |    18  |     0  |       -  |    18 | 100.00% |    100.00% |
| regression           |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| stack-overflow       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| stack-size           |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| stc                  |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| template-literal-type |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| top-level-await      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts-import-type       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| tsx                  |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| type-alias           |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| type-arguments       |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| types                |    30  |     0  |       -  |    30 | 100.00% |    100.00% |
| v4                   |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| variable-declarator  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| variance-annotations |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| vercel               |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |   535  |     1  |       2  |   536 |  99.81% |     99.44% |
<!-- end:swc-results -->

### test262
<!-- begin:test262-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| early                |   668  |     0  |       -  |   668 | 100.00% |    100.00% |
| fail                 |   721  |     0  |       8  |   721 | 100.00% |     98.90% |
| pass                 |  1866  |     0  |     117  |  1866 | 100.00% |     94.10% |
| pass-explicit        |  1919  |     0  |      64  |  1919 | 100.00% |     96.77% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |  5174  |     0  |     189  |  5174 | 100.00% |     96.48% |
<!-- end:test262-results -->
