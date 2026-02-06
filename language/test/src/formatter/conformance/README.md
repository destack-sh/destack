# Formatter Conformance Tests

Formatter conformance tests measure how close Destack formatter behavior is to external formatter ecosystems.

## Status

The pass rate intentionally excludes explicitly ignored tests.
Ignored tests track intentional differences and unsupported or out of scope behaviors.

<!-- (results are automatically updated by the formatter conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Ignored | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| biome    |   316  |  1420  |     -  |  1736 |  18.20% |
| oxfmt    |    11  |   122  |     -  |   133 |   8.27% |
| prettier |  1929  |  1299  |     -  |  3228 |  59.78% |
|----------|--------|--------|---------|-------|---------|
| total    |  2256  |  2841  |      -  |  5097 |  44.26% |

Total Blended Pass Rate: **44.26%**
<!-- end:summary-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |    98  |    71  |       -  |   169 |  57.99% |
| jsx                  |    11  |     8  |       -  |    19 |  57.89% |
| prettier             |   152  |  1318  |       -  |  1470 |  10.34% |
| ts                   |    52  |    23  |       -  |    75 |  69.33% |
| tsx                  |     3  |     0  |       -  |     3 | 100.00% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   316  |  1420  |       -  |  1736 |  18.20% |
<!-- end:biome-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| flow                 |   784  |   631  |       -  |  1415 |  55.41% |
| flow-repo            |     7  |     7  |       -  |    14 |  50.00% |
| js                   |   610  |   428  |       -  |  1038 |  58.77% |
| jsx                  |     1  |    62  |       -  |    63 |   1.59% |
| misc                 |    32  |    12  |       -  |    44 |  72.73% |
| typescript           |   495  |   159  |       -  |   654 |  75.69% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  1929  |  1299  |       -  |  3228 |  59.76% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |     9  |    65  |       -  |    74 |  12.16% |
| ts                   |     2  |    57  |       -  |    59 |   3.39% |
|----------------------|--------|--------|---------|-------|---------|
| total                |    11  |   122  |       -  |   133 |   8.27% |
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
