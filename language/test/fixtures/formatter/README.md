# Formatter Test Fixtures

This directory contains formatter test fixtures with two types of tests:

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

## Running Tests

```bash
just test-formatter
```

To run a specific test:
```bash
just test-formatter spacing  # runs tests matching "spacing"
```
