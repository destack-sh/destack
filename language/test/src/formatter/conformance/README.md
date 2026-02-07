# Formatter Conformance Tests

Formatter conformance tests measure how close Destack formatter behavior is to external formatter ecosystems.

## Status

The pass rate intentionally excludes explicitly ignored tests.
Ignored tests track intentional differences and unsupported or out of scope behaviors.

<!-- (results are automatically updated by the formatter conformance test runner) -->
<!-- begin:summary-results -->
| Suite    | Passed | Failed | Ignored | Total |  Rate   |
|:---------|-------:|-------:|--------:|------:|--------:|
| biome    |   544  |  1192  |     -  |  1736 |  31.34% |
| oxfmt    |    86  |    41  |     6  |   127 |  67.72% |
| prettier |  1934  |  1294  |     -  |  3228 |  59.93% |
|----------|--------|--------|---------|-------|---------|
| total    |  2564  |  2527  |      6  |  5091 |  50.36% |

Total Blended Pass Rate: **50.36%**
<!-- end:summary-results -->

### biome
<!-- begin:biome-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |    97  |    72  |       -  |   169 |  57.40% |
| jsx                  |    12  |     7  |       -  |    19 |  63.16% |
| prettier             |   385  |  1085  |       -  |  1470 |  26.19% |
| ts                   |    48  |    27  |       -  |    75 |  64.00% |
| tsx                  |     2  |     1  |       -  |     3 |  66.67% |
|----------------------|--------|--------|---------|-------|---------|
| total                |   544  |  1192  |       -  |  1736 |  31.34% |
<!-- end:biome-results -->

### prettier
<!-- begin:prettier-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| flow                 |   786  |   629  |       -  |  1415 |  55.55% |
| flow-repo            |     7  |     7  |       -  |    14 |  50.00% |
| js                   |   626  |   412  |       -  |  1038 |  60.31% |
| jsx                  |     1  |    62  |       -  |    63 |   1.59% |
| misc                 |    33  |    11  |       -  |    44 |  75.00% |
| typescript           |   481  |   173  |       -  |   654 |  73.55% |
|----------------------|--------|--------|---------|-------|---------|
| total                |  1934  |  1294  |       -  |  3228 |  59.91% |
<!-- end:prettier-results -->

### oxfmt
<!-- begin:oxfmt-results -->
| Category             | Passed | Failed | Ignored | Total |  Rate   |
|:---------------------|-------:|-------:|--------:|------:|--------:|
| js                   |    54  |    18  |       2  |    72 |  75.00% |
| ts                   |    32  |    23  |       4  |    55 |  58.18% |
|----------------------|--------|--------|---------|-------|---------|
| total                |    86  |    41  |       6  |   127 |  67.72% |
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
