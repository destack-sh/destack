# Formatter Conformance Tests

Formatter conformance tests measure how close Destack formatter behavior is to external formatter ecosystems.

## Compliance Policy

Formatter conformance uses a tiered target model.

The tier definitions are below.
1. `hard`: `oxfmt` parity for supported JS/TS/JSX/TSX syntax.
2. `hard`: Destack owned formatter spec and fixture behavior for DS and TS++ syntax.
3. `soft`: `prettier` and `biome` parity for gap discovery and prioritization.

Conflict resolution follows this order.
1. Destack language spec and formatter fixtures for DS and TS++ behavior.
2. `oxfmt` for shared JS/TS behavior.
3. `prettier` and `biome` as advisory when they conflict with `oxfmt`.

Known failures and ignored files have strict meanings.
1. `*-known-failures.txt`: active gaps we intend to burn down.
2. `*-ignored.txt`: intentional divergence or unsupported scope only.
3. Flow and `flow-repo` fixtures are unsupported language scope and stay in `ignored`.

Runner output and README summary rows include a suite tier label.
`hard` means release blocking for that suite's in-scope coverage.
`soft` means informational and prioritization signal, not a hard merge gate.

## Status

The pass rate intentionally excludes explicitly ignored tests.
Ignored tests track intentional differences and unsupported or out of scope behaviors.

<!-- (results are automatically updated by the formatter conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Tier | Passed | Failed | Ignored | Total |  Rate   |
|:---------|:-----|-------:|-------:|--------:|------:|--------:|
| biome    | soft |   612  |  1115  |     9  |  1727 |  35.44% |
| oxfmt    | hard |   129  |     0  |     6  |   129 | 100.00% |
| prettier | soft |  1173  |   619  |  1436  |  1792 |  65.49% |
|----------|------|--------|--------|---------|-------|---------|
| total    | -    |  1914  |  1734  |   1451  |  3648 |  52.47% |

Total Blended Pass Rate: **52.47%**
<!-- end:summary-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |   105  |    64  |       -  |   169 |  62.13% |
| jsx                  |    12  |     7  |       -  |    19 |  63.16% |
| prettier             |   444  |  1017  |       9  |  1461 |  30.39% |
| ts                   |    49  |    26  |       -  |    75 |  65.33% |
| tsx                  |     2  |     1  |       -  |     3 |  66.67% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   612  |  1115  |       9  |  1727 |  35.44% |
<!-- end:biome-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| flow                 |     0  |     0  |    1415  |     0 | 100.00% |
| flow-repo            |     0  |     0  |      14  |     0 | 100.00% |
| js                   |   641  |   396  |       1  |  1037 |  61.81% |
| jsx                  |     2  |    61  |       -  |    63 |   3.17% |
| misc                 |    33  |    11  |       -  |    44 |  75.00% |
| typescript           |   497  |   151  |       6  |   648 |  76.70% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  1173  |   619  |    1436  |  1792 |  65.46% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |    73  |     0  |       2  |    73 | 100.00% |
| ts                   |    56  |     0  |       4  |    56 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   129  |     0  |       6  |   129 | 100.00% |
<!-- end:oxfmt-results -->

## Running

Run all formatter conformance suites.

```sh
cargo test --release --test formatter-conformance
just language/test-formatter-conformance
```

Run one formatter conformance suite.

```sh
cargo test --release --test formatter-conformance -- --biome
cargo test --release --test formatter-conformance -- --prettier
cargo test --release --test formatter-conformance -- --oxfmt
```

Update known failure baselines.

```sh
cargo test --release --test formatter-conformance -- --update-known-failures
just language/test-formatter-conformance-update
```
