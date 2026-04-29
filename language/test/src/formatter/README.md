# Formatter Tests

The formatter test suite includes two types of tests:

1. **Transform tests** (`.md` files): Verify formatting produces correct output from input
2. **Roundtrip tests** (`.ds/.d.ds/.js/.jsx/.ts/.tsx/.d.ts` files): Verify formatted code remains stable

## Fixtures

- `fixtures/formatter/transform/*.md` - MDTest transform tests
- `fixtures/formatter/roundtrip/*.{ds,d.ds,js,jsx,ts,tsx,d.ts}` - Roundtrip stability tests

Roundtrip fixtures using JS, JSX, TS, and TSX should be canonicalized with the shared formatter conformance baseline before being added.
Use formatter-compatible options so the file content is the expected baseline for idempotence.

## Running

Run these commands from `language/`.

```bash
# Run all formatter tests
just test-formatter

# Run specific tests
cargo test --release --test formatter -- spacing     # tests matching "spacing"
cargo test --release --test formatter -- roundtrip   # only roundtrip tests
cargo test --release --test formatter -- transform   # only transform tests
just test-conformance-formatter    # external suite conformance harness
```
