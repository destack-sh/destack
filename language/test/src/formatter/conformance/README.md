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
| oxfmt    |   129  |     0  |     6  |   129 | 100.00% |  95.56% |
| prettier |  1524  |    43  |  1663  |  1567 |  97.26% |  47.18% |
|----------|--------|--------|---------|-------|---------|------------|
| total    |  1653  |    43  |   1669  |  1696 |  97.46% |     49.12% |

Total Blended Pass Rate: **97.46%** (49.12% incl. ignored)
<!-- end:summary-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| flow                 |     0  |     0  |    1415  |     0 | 100.00% |      0.00% |
| flow-repo            |     0  |     0  |      14  |     0 | 100.00% |      0.00% |
| js                   |   830  |    20  |     190  |   850 |  97.65% |     79.81% |
| jsx                  |    53  |     3  |       7  |    56 |  94.64% |     84.13% |
| misc                 |    42  |     0  |       2  |    42 | 100.00% |     95.45% |
| typescript           |   599  |    20  |      35  |   619 |  96.77% |     91.59% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |  1524  |    43  |    1663  |  1567 |  97.26% |     47.18% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| js                   |    73  |     0  |       2  |    73 | 100.00% |     97.33% |
| ts                   |    56  |     0  |       4  |    56 | 100.00% |     93.33% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |   129  |     0  |       6  |   129 | 100.00% |     95.56% |
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
