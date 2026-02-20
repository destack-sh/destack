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
| Suite    | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------|-------:|-------:|--------:|------:|--------:|-----------:|
| oxfmt    |    98  |    31  |     6  |   129 |  75.97% |  72.59% |
| prettier |  1248  |   262  |  1720  |  1510 |  82.65% |  38.64% |
|----------|--------|--------|---------|-------|---------|------------|
| total    |  1346  |   293  |   1726  |  1639 |  82.12% |     40.00% |

Total Blended Pass Rate: **82.12%** (40.00% incl. ignored)
<!-- end:summary-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| flow                 |     0  |     0  |    1415  |     0 | 100.00% |      0.00% |
| flow-repo            |     0  |     0  |      14  |     0 | 100.00% |      0.00% |
| js                   |   629  |   155  |     256  |   784 |  80.23% |     60.48% |
| jsx                  |    39  |    18  |       6  |    57 |  68.42% |     61.90% |
| misc                 |    42  |     0  |       2  |    42 | 100.00% |     95.45% |
| typescript           |   538  |    89  |      27  |   627 |  85.81% |     82.26% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |  1248  |   262  |    1720  |  1510 |  82.65% |     38.64% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| js                   |    51  |    22  |       2  |    73 |  69.86% |     68.00% |
| ts                   |    47  |     9  |       4  |    56 |  83.93% |     78.33% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |    98  |    31  |       6  |   129 |  75.97% |     72.59% |
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
