# Formatter Tests

The formatter test suite includes two types of tests:

1. **Transform tests** (`.md` files): Verify formatting produces correct output from input
2. **Roundtrip tests** (`.ds` files): Verify formatted code remains stable

## Fixtures

- `fixtures/formatter/transform/*.md` - MDTest transform tests
- `fixtures/formatter/roundtrip/*.ds` - Roundtrip stability tests

## Running

```bash
# Run all formatter tests
just test-formatter

# Run specific tests
just test-formatter spacing        # tests matching "spacing"
just test-formatter roundtrip      # only roundtrip tests
just test-formatter transform      # only transform tests
```
