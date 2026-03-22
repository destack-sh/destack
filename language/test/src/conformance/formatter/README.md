# Formatter Conformance Tests

Formatter conformance tests measure how close Destack formatter behavior is to external formatter ecosystems.

## Status

The pass rate intentionally excludes explicitly ignored tests.
Ignored tests track intentional differences and unsupported / out of scope behaviors.
For Prettier, this is a lot, but surprisingly there is a _lot_ of Flow stuff, some IDE-specific features like cursors, and quite a few deliberate error cases.

<!-- (results are automatically updated by the formatter conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------|-------:|-------:|--------:|------:|--------:|-----------:|
| oxfmt    |   135  |     0  |     -  |   135 | 100.00% | 100.00% |
| prettier |  1554  |     0  |  1676  |  1554 | 100.00% |  48.11% |
|----------|--------|--------|---------|-------|---------|------------|
| total    |  1689  |     0  |   1676  |  1689 | 100.00% |     50.19% |

Total Blended Pass Rate: **100.00%** (50.19% incl. ignored)
<!-- end:summary-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| flow-repo/ambient_declarations |     0  |     0  |      14  |     0 | 100.00% |      0.00% |
| flow/_errors_        |     0  |     0  |      12  |     0 | 100.00% |      0.00% |
| flow/all             |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/annotation      |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/array-comments  |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/array-union     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/as-const        |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/as-satisfies-expression |     0  |     0  |       9  |     0 | 100.00% |      0.00% |
| flow/assignments     |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/await           |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/break-calls     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/class           |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/class-field     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/comments        |     0  |     0  |      15  |     0 | 100.00% |      0.00% |
| flow/component       |     0  |     0  |       5  |     0 | 100.00% |      0.00% |
| flow/conditional-types |     0  |     0  |       8  |     0 | 100.00% |      0.00% |
| flow/conditional-types-comments |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/cursor          |     0  |     0  |       8  |     0 | 100.00% |      0.00% |
| flow/declare-class   |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/declare-function |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/declare-interface |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/declare-opaque-type |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/declare-type    |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/declare-variable |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/decorator       |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/empty-parameters-with-arrow-function |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/enums           |     0  |     0  |      16  |     0 | 100.00% |      0.00% |
| flow/enums-unknown-members |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/exact-object    |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/export          |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/flow-babel-only |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/flow-intersection |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/flow-repo       |     0  |     0  |    1169  |     0 | 100.00% |      0.00% |
| flow/function        |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/function-parentheses |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/function-single-destructuring |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/function-type-param |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/generic         |     0  |     0  |       9  |     0 | 100.00% |      0.00% |
| flow/hook            |     0  |     0  |       5  |     0 | 100.00% |      0.00% |
| flow/ignore          |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/implements      |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/import          |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/import-type-specifier |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/indexed-access  |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/interface-types |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/internal-slot   |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/intersection    |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/jsx             |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/keyof           |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/last-argument-expansion |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/literal         |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/mapped-types    |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/match           |     0  |     0  |      15  |     0 | 100.00% |      0.00% |
| flow/maybe           |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/method          |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/method-chain    |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/mixins          |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/no-semi         |     0  |     0  |       5  |     0 | 100.00% |      0.00% |
| flow/object-comment  |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/object-inexact  |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/object-multiline |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/object-order    |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/object-property-comment |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/optional-indexed-access |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/optional-type-name |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/parameter-with-type |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/parenthesis-return-type |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/private-class-fields |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/proto-props     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/quote-props     |     0  |     0  |       6  |     0 | 100.00% |      0.00% |
| flow/range           |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/records         |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/return-arrow    |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/template        |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/ternary         |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/this-annotation |     0  |     0  |       7  |     0 | 100.00% |      0.00% |
| flow/tuple-types     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/tuples          |     0  |     0  |       5  |     0 | 100.00% |      0.00% |
| flow/type-alias      |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| flow/type-cast       |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/type-declarations |     0  |     0  |       7  |     0 | 100.00% |      0.00% |
| flow/type-guards     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/type-parameters |     0  |     0  |       6  |     0 | 100.00% |      0.00% |
| flow/type-spread     |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/typeapp-call    |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/typeof          |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/union           |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| flow/union_intersection |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| flow/variance        |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/_errors_          |    27  |     0  |      17  |    27 | 100.00% |     61.36% |
| js/array-spread      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/arrays            |    18  |     0  |       -  |    18 | 100.00% |    100.00% |
| js/arrow-call        |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/arrows            |    27  |     0  |       -  |    27 | 100.00% |    100.00% |
| js/arrows-bind       |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/assignment        |    28  |     0  |       -  |    28 | 100.00% |    100.00% |
| js/assignment-comments |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/assignment-expression |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/async             |    10  |     0  |       -  |    10 | 100.00% |    100.00% |
| js/async-do-expressions |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/babel-plugins     |    31  |     0  |      13  |    31 | 100.00% |     70.45% |
| js/big-int           |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/binary-expressions |    21  |     0  |       -  |    21 | 100.00% |    100.00% |
| js/binary_math       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/bind-expressions  |     0  |     0  |       6  |     0 | 100.00% |      0.00% |
| js/bracket-spacing   |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/break-calls       |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/call              |    13  |     0  |       -  |    13 | 100.00% |    100.00% |
| js/chain-expression  |    15  |     0  |       -  |    15 | 100.00% |    100.00% |
| js/class-comment     |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/class-extends     |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/class-static-block |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/classes           |    27  |     0  |       -  |    27 | 100.00% |    100.00% |
| js/classes-private-fields |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/comments          |    62  |     0  |       2  |    62 | 100.00% |     96.88% |
| js/comments-closure-typecast |    17  |     0  |       6  |    17 | 100.00% |     73.91% |
| js/comments-pipeline-own-line |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/computed-props    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/conditional       |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/cursor            |     6  |     0  |      26  |     6 | 100.00% |     18.75% |
| js/decorator-auto-accessors |    11  |     0  |       -  |    11 | 100.00% |    100.00% |
| js/decorators        |    14  |     0  |       -  |    14 | 100.00% |    100.00% |
| js/decorators-export |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/deferred-import-evaluation |     0  |     0  |       6  |     0 | 100.00% |      0.00% |
| js/destructuring     |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/destructuring-ignore |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/destructuring-private-fields |     0  |     0  |       7  |     0 | 100.00% |      0.00% |
| js/directives        |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| js/discard-binding   |     8  |     0  |       7  |     8 | 100.00% |     53.33% |
| js/do                |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| js/dynamic-import    |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/empty-paren-comment |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/empty-statement   |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/end-of-line       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/eol               |     0  |     0  |       4  |     0 | 100.00% |      0.00% |
| js/es6modules        |    10  |     0  |       -  |    10 | 100.00% |    100.00% |
| js/explicit-resource-management |    26  |     0  |       4  |    26 | 100.00% |     86.67% |
| js/export            |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/export-default    |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| js/export-star       |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/expression_statement |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/for               |    10  |     0  |       1  |    10 | 100.00% |     90.91% |
| js/for-await         |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/for-of            |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/function          |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| js/function-comments |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/function-first-param |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/function-single-destructuring |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/functional-composition |    13  |     0  |       -  |    13 | 100.00% |    100.00% |
| js/generator         |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/identifier        |     0  |     0  |       4  |     0 | 100.00% |      0.00% |
| js/if                |    12  |     0  |       -  |    12 | 100.00% |    100.00% |
| js/ignore            |    13  |     0  |       -  |    13 | 100.00% |    100.00% |
| js/import            |    10  |     0  |       1  |    10 | 100.00% |     90.91% |
| js/import-assertions |     0  |     0  |      13  |     0 | 100.00% |      0.00% |
| js/import-attributes |    15  |     0  |       -  |    15 | 100.00% |    100.00% |
| js/import-meta       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/in                |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/invalid-code      |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/label             |     2  |     0  |       1  |     2 | 100.00% |     66.67% |
| js/last-argument-expansion |    18  |     0  |       -  |    18 | 100.00% |    100.00% |
| js/line              |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/line-suffix-boundary |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/literal           |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/literal-numeric-separator |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/logical-assignment |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/logical-expressions |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/member            |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/method-chain      |    35  |     0  |       -  |    35 | 100.00% |    100.00% |
| js/module-blocks     |     1  |     0  |       4  |     1 | 100.00% |     20.00% |
| js/module-string-names |     1  |     0  |       1  |     1 | 100.00% |     50.00% |
| js/multiparser-comments |     2  |     0  |       1  |     2 | 100.00% |     66.67% |
| js/multiparser-css   |    16  |     0  |       -  |    16 | 100.00% |    100.00% |
| js/multiparser-graphql |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| js/multiparser-html  |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/multiparser-invalid |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/multiparser-markdown |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/multiparser-text  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/new-expression    |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/new-target        |     1  |     0  |       1  |     1 | 100.00% |     50.00% |
| js/newline           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/no-semi           |    16  |     0  |       1  |    16 | 100.00% |     94.12% |
| js/no-semi-babylon-extensions |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/non-strict        |     2  |     0  |       1  |     2 | 100.00% |     66.67% |
| js/nullish-coalescing |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/numeric-separators |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/object-colon-bug  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/object-multiline  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/object-prop-break-in |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/object-property-comment |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/object-property-ignore |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/objects           |     7  |     0  |       1  |     7 | 100.00% |     87.50% |
| js/optional-catch-binding |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/optional-chaining |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/optional-chaining-assignment |     6  |     0  |       8  |     6 | 100.00% |     42.86% |
| js/partial-application |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/performance       |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/pipeline-operator |     0  |     0  |       4  |     0 | 100.00% |      0.00% |
| js/preserve-line     |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/private-in        |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/quote-props       |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| js/quotes            |     3  |     0  |       1  |     3 | 100.00% |     75.00% |
| js/range             |     2  |     0  |      36  |     2 | 100.00% |      5.26% |
| js/regex             |     4  |     0  |       1  |     4 | 100.00% |     80.00% |
| js/require           |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/require-amd       |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/reserved-word     |     1  |     0  |       2  |     1 | 100.00% |     33.33% |
| js/rest              |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/return            |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/return-outside-function |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/sequence-break    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/sequence-expression |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| js/shebang           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/sloppy-mode       |     3  |     0  |       3  |     3 | 100.00% |     50.00% |
| js/source-phase-imports |     3  |     0  |       6  |     3 | 100.00% |     33.33% |
| js/spread            |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/strings           |     4  |     0  |       1  |     4 | 100.00% |     80.00% |
| js/switch            |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/tab-width         |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/template          |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| js/template-align    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/template-literals |    11  |     0  |       -  |    11 | 100.00% |    100.00% |
| js/ternaries         |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| js/test-declarations |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| js/throw_expressions |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/throw_statement   |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/top-level-await   |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| js/trailing-comma    |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/try               |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/unary             |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/unary-expression  |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/unicode           |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/update-expression |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/v8_intrinsic      |     0  |     0  |       2  |     0 | 100.00% |      0.00% |
| js/variable_declarator |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/while             |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/with              |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| js/yield             |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| jsx/attr-element     |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/binary-expressions |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/comments         |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| jsx/cursor           |     0  |     0  |       6  |     0 | 100.00% |      0.00% |
| jsx/deprecated-jsx-bracket-same-line-option |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/do               |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| jsx/embed            |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/escape           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/expression-with-types |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/fbt              |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/fragment         |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/ignore           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/jsx              |    20  |     0  |       -  |    20 | 100.00% |    100.00% |
| jsx/last-line        |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/multiline-assign |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/namespace        |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/newlines         |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/optional-chaining |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/parentheses      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/significant-space |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/single-attribute-per-line |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/split-attrs      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/spread           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/stateless-arrow-fn |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/template         |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| jsx/text-wrap        |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| jsx/top-level-await  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| misc/babel-redirect-to-babel-flow |    16  |     0  |       -  |    16 | 100.00% |    100.00% |
| misc/check-ignore-pragma |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| misc/embedded-language-formatting |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| misc/insert-pragma   |    12  |     0  |       -  |    12 | 100.00% |    100.00% |
| misc/parser-inference |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| misc/require-pragma  |     4  |     0  |       2  |     4 | 100.00% |     66.67% |
| misc/shared-fixtures |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/_errors_  |    15  |     0  |      13  |    15 | 100.00% |     53.57% |
| typescript/abstract-class |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/abstract-construct-types |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/abstract-property |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/ambient   |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/angular-component-examples |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/argument-expansion |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/array     |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/arrow     |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| typescript/arrows    |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/as        |    15  |     0  |       -  |    15 | 100.00% |    100.00% |
| typescript/assert    |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/assignment |    14  |     0  |       -  |    14 | 100.00% |    100.00% |
| typescript/bigint    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/binary-expressions |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/break-calls |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/call      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/call-signature |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/cast      |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| typescript/catch-clause |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/chain-expression |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| typescript/class     |    15  |     0  |       -  |    15 | 100.00% |    100.00% |
| typescript/class-and-interface |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/class-comment |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/classes   |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/comments  |    25  |     0  |       -  |    25 | 100.00% |    100.00% |
| typescript/comments-2 |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/compiler  |    17  |     0  |       1  |    17 | 100.00% |     94.44% |
| typescript/conditional-types |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| typescript/conformance |   124  |     0  |       2  |   124 | 100.00% |     98.41% |
| typescript/const     |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/cursor    |     0  |     0  |      10  |     0 | 100.00% |      0.00% |
| typescript/custom    |    19  |     0  |       -  |    19 | 100.00% |    100.00% |
| typescript/d-ts-files |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/declare   |    10  |     0  |       -  |    10 | 100.00% |    100.00% |
| typescript/decorator-auto-accessors |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/decorators |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| typescript/decorators-ts |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| typescript/definite  |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/destructuring |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/edge-cases |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/end-of-line |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/enum      |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/error-recovery |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/explicit-resource-management |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/export    |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| typescript/export-default |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/function  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/function-type |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/functional-composition |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/generic   |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/import-export |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/import-require |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/import-type |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| typescript/index-signature |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/infer-extends |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/instantiation-expression |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| typescript/interface |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| typescript/interface2 |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| typescript/intersection |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| typescript/intrinsic |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/key-remapping-in-mapped-types |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/keyof     |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/keyword-types |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/keywords  |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/last-argument-expansion |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/literal   |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/mapped-type |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| typescript/method    |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| typescript/method-chain |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| typescript/module    |     5  |     0  |       1  |     5 | 100.00% |     83.33% |
| typescript/multiparser-css |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/namespace |     0  |     0  |       1  |     0 | 100.00% |      0.00% |
| typescript/never     |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/new       |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/no-semi   |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/non-null  |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/nosemi    |     4  |     0  |       1  |     4 | 100.00% |     80.00% |
| typescript/object-multiline |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/optional-call |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/optional-chaining |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/optional-method |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/optional-type |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/optional-variance |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/override-modifiers |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/predicate-types |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/prettier-ignore |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| typescript/private-fields-in-in |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/property-signature |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/quote-props |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/range     |     0  |     0  |       3  |     0 | 100.00% |      0.00% |
| typescript/readonly  |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/rest      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/rest-type |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/satisfies-operators |    14  |     0  |       2  |    14 | 100.00% |     87.50% |
| typescript/semi      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/static-blocks |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/symbol    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/template-literal-types |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/template-literals |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/ternaries |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/test-declarations |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/top-level-await |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/trailing-comma |     3  |     0  |       1  |     3 | 100.00% |     75.00% |
| typescript/tsx       |     9  |     0  |       -  |     9 | 100.00% |    100.00% |
| typescript/tuple     |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| typescript/type-alias |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| typescript/type-arguments-bit-shift-left-like |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| typescript/type-member-get-set |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/type-only-module-specifiers |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/type-parameters-arguments |    17  |     0  |       -  |    17 | 100.00% |    100.00% |
| typescript/typeof    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/typeof-this |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| typescript/typescript-babel-only |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| typescript/typescript-only |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| typescript/union     |    23  |     0  |       -  |    23 | 100.00% |    100.00% |
| typescript/unique-symbol |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/unknown   |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/update-expression |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| typescript/webhost   |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |  1554  |     0  |    1676  |  1554 | 100.00% |     48.11% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| js/arguments         |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/array-assignment  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/array-pattern     |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/assignments       |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| js/await-expression  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/awaits            |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/call-expression   |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/calls             |     6  |     0  |       -  |     6 | 100.00% |    100.00% |
| js/class             |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/comments          |    16  |     0  |       -  |    16 | 100.00% |    100.00% |
| js/computed-members  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/conditional       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/crlf              |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/function-compositions |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/if                |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/ignore            |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/import-expressions |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/jsx               |     7  |     0  |       -  |     7 | 100.00% |    100.00% |
| js/logicals          |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/member-chains     |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| js/new-expression    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/private-fields    |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/quote-props       |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/semicolons        |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/sequence-expression |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/template-literals |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| js/typecast          |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| js/unicode           |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| js/yield             |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/arguments         |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| ts/arrow-function    |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| ts/assignments       |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| ts/class             |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| ts/comments          |     8  |     0  |       -  |     8 | 100.00% |    100.00% |
| ts/conditional-type  |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/directives        |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/function-parameters |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/functions         |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/mapped-type       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/member-chains     |     4  |     0  |       -  |     4 | 100.00% |    100.00% |
| ts/method-signatures |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/new-expression    |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| ts/objects           |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/parameters        |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| ts/parenthesis       |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/quote-props       |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| ts/semi              |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/semicolons        |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/static-members    |     3  |     0  |       -  |     3 | 100.00% |    100.00% |
| ts/template-literal-type |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/type-literal      |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/type-parameters   |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
| ts/union             |     5  |     0  |       -  |     5 | 100.00% |    100.00% |
| ts/union-type        |     1  |     0  |       -  |     1 | 100.00% |    100.00% |
| ts/variable-declarations |     2  |     0  |       -  |     2 | 100.00% |    100.00% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |   135  |     0  |       -  |   135 | 100.00% |    100.00% |
<!-- end:oxfmt-results -->

## Running

Run all formatter conformance suites.

```sh
cargo test --release --test conformance-formatter
just language/test-conformance-formatter
```

Run one formatter conformance suite.

```sh
cargo test --release --test conformance-formatter -- --prettier
cargo test --release --test conformance-formatter -- --oxfmt
```

Update known failure baselines.

```sh
cargo test --release --test conformance-formatter -- --update-known-failures
just language/test-conformance-formatter-update
```
