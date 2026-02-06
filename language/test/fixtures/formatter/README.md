# Formatter Test Fixtures

This directory contains formatter test fixtures with four fixture groups:

## Transform Tests (`.md` files)

MDTest format for verifying formatting transformations. Each test specifies input code and expected formatted output.

```markdown
## Section Name

### test name

```ds
input code here (possibly messy)
```

```expected
expected formatted output
```
```


## Roundtrip Tests (`.ds` files)

Pre-formatted code that should remain unchanged when formatted.

Standard `.ds` files with properly formatted code.

## Smoke Tests (`smoke/**/input.*`)

Smoke fixtures are small curated local cases for output parity or idempotence checks.

Each test case is a directory with an `input.*` file and optional `expected.*` file of the same extension.

When `expected.*` exists, the formatter output must match it exactly.

When `expected.*` is missing, the test runs idempotence only, and requires `fmt(fmt(input)) == fmt(input)`.

## External Conformance Suites (`conformance/staging/**`)

External corpora fetched from Biome, Prettier, and oxfmt live under `conformance/staging/`.
These are used by the dedicated `formatter-conformance` test binary and not by `--test formatter`.

## Running Tests

```bash
just test-formatter
```

To fetch upstream formatter conformance sources:

```bash
just language/install-formatter-conformance
```

To run a specific test:
```bash
just test-formatter spacing  # runs tests matching "spacing"
```
