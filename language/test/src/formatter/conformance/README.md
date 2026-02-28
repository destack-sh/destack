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
| prettier |  1513  |    54  |  1663  |  1567 |  96.55% |  46.84% |
|----------|--------|--------|---------|-------|---------|------------|
| total    |  1642  |    54  |   1669  |  1696 |  96.82% |     48.80% |

Total Blended Pass Rate: **96.82%** (48.80% incl. ignored)
<!-- end:summary-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:---------------------|-------:|-------:|--------:|------:|--------:|-----------:|
| flow                 |     0  |     0  |    1415  |     0 | 100.00% |      0.00% |
| flow-repo            |     0  |     0  |      14  |     0 | 100.00% |      0.00% |
| js                   |   823  |    27  |     190  |   850 |  96.82% |     79.13% |
| jsx                  |    48  |     8  |       7  |    56 |  85.71% |     76.19% |
| misc                 |    42  |     0  |       2  |    42 | 100.00% |     95.45% |
| typescript           |   600  |    19  |      35  |   619 |  96.93% |     91.74% |
|----------------------|--------|--------|---------|-------|---------|------------|
| total                |  1513  |    54  |    1663  |  1567 |  96.55% |     46.84% |
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
