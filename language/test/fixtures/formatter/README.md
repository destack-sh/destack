# Formatter Test Fixtures

This directory contains formatter test fixtures with two fixture groups: transform and roundtrip tests. 
They test exactly what the name implies.

## Transform Tests (`.md` files)

MDTest format for verifying formatting transformations. Each test specifies input code and expected formatted output.

````markdown
## Section Name

### test name

```ds
input code here (possibly messy)
```

```expected
expected formatted output
```
````


## Roundtrip Tests (`roundtrip/*.{ds,d.ds,js,jsx,ts,tsx,d.ts}`)

Pre-formatted code that should remain unchanged when formatted.

Roundtrip fixtures cover canonicalized shared JS, JSX, TS, and TSX code as well as reviewed Destack local syntax.

## Running Tests

Run these commands from `language/`.

```bash
just test-formatter
```

To fetch upstream formatter conformance sources:

```bash
just install-conformance-formatter
```

To run a specific test:
```bash
cargo test --release --test formatter -- spacing  # runs tests matching "spacing"
```
