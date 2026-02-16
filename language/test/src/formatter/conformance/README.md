# Formatter Conformance Tests

Formatter conformance tests measure how close Destack formatter behavior is to external formatter ecosystems.

## Compliance Policy

Formatter conformance uses a unified target model.

The target definitions are below.
1. Destack owned formatter spec and fixture behavior for DS and TS++ syntax.
2. `oxfmt` parity for supported JS/TS/JSX/TSX syntax.
3. `prettier` parity for supported JS/TS/JSX/TSX syntax.

Conflict resolution follows this order.
1. Destack language spec and formatter fixtures for DS and TS++ behavior.
2. Require parity with both `oxfmt` and `prettier` for shared supported syntax.
3. If upstream suites genuinely disagree, encode the chosen behavior in Destack fixtures and document the other case in `*-ignored.txt`.

Known failures and ignored files have strict meanings.
1. `*-known-failures.txt`: active gaps we intend to burn down.
2. `*-ignored.txt`: intentional divergence or unsupported / out of scope behaviors.
3. Flow and `flow-repo` fixtures are unsupported language scope and stay in `ignored`.

Runner output and README summary rows report per-suite conformance without tier labels.
Each suite is treated as release relevant for its supported syntax scope.

## Status

The pass rate intentionally excludes explicitly ignored tests.
Ignored tests track intentional differences and unsupported / out of scope behaviors.
(Mostly flow stuff, IDE-specific features like cursors, and some deliberate error cases.)

<!-- (results are automatically updated by the formatter conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Ignored | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| oxfmt    |   103  |    26  |     6  |   129 |  79.84% |
| prettier |  1327  |   341  |  1560  |  1668 |  79.56% |
|----------|--------|--------|---------|-------|---------|
| total    |  1430  |   367  |   1566  |  1797 |  79.58% |

Total Blended Pass Rate: **79.58%**
<!-- end:summary-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| flow                 |     0  |     0  |    1415  |     0 | 100.00% |
| flow-repo            |     0  |     0  |      14  |     0 | 100.00% |
| js                   |   698  |   244  |      96  |   942 |  74.10% |
| jsx                  |    38  |    19  |       6  |    57 |  66.67% |
| misc                 |    42  |     0  |       2  |    42 | 100.00% |
| typescript           |   549  |    78  |      27  |   627 |  87.56% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  1327  |   341  |    1560  |  1668 |  79.56% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |    57  |    16  |       2  |    73 |  78.08% |
| ts                   |    46  |    10  |       4  |    56 |  82.14% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   103  |    26  |       6  |   129 |  79.84% |
<!-- end:oxfmt-results -->

## Running

Run all formatter conformance suites.

```sh
cargo test --release --test formatter-conformance
just language/test-formatter-conformance
```

Run one formatter conformance suite.

```sh
cargo test --release --test formatter-conformance -- --prettier
cargo test --release --test formatter-conformance -- --oxfmt
```

Update known failure baselines.

```sh
cargo test --release --test formatter-conformance -- --update-known-failures
just language/test-formatter-conformance-update
```
