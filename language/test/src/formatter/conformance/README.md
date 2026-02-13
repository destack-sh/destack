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
| biome    | soft |   643  |  1084  |     9  |  1727 |  37.23% |
| oxfmt    | hard |   125  |     4  |     6  |   129 |  96.90% |
| prettier | soft |  1285  |   174  |  1769  |  1459 |  88.07% |
|----------|------|--------|--------|---------|-------|---------|
| total    | -    |  2053  |  1262  |   1784  |  3315 |  61.93% |

Total Blended Pass Rate: **61.93%**
<!-- end:summary-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |   120  |    49  |       -  |   169 |  71.01% |
| jsx                  |    13  |     6  |       -  |    19 |  68.42% |
| prettier             |   455  |  1006  |       9  |  1461 |  31.14% |
| ts                   |    52  |    23  |       -  |    75 |  69.33% |
| tsx                  |     3  |     0  |       -  |     3 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   643  |  1084  |       9  |  1727 |  37.23% |
<!-- end:biome-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| flow                 |     0  |     0  |    1415  |     0 | 100.00% |
| flow-repo            |     0  |     0  |      14  |     0 | 100.00% |
| js                   |   696  |    91  |     251  |   787 |  88.44% |
| jsx                  |     2  |    26  |      35  |    28 |   7.14% |
| misc                 |    42  |     0  |       2  |    42 | 100.00% |
| typescript           |   545  |    57  |      52  |   602 |  90.53% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  1285  |   174  |    1769  |  1459 |  88.07% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |    69  |     4  |       2  |    73 |  94.52% |
| ts                   |    56  |     0  |       4  |    56 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   125  |     4  |       6  |   129 |  96.90% |
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
