# Formatter Tests

The formatter test suite includes two types of tests:

1. **Transform tests** (`.md` files): Verify formatting produces correct output from input
2. **Roundtrip tests** (`.ds` files): Verify formatted code remains stable
3. **Smoke tests** (`input.*` fixtures): Verify compatibility and idempotence on curated local cases

## Fixtures

- `fixtures/formatter/transform/*.md` - MDTest transform tests
- `fixtures/formatter/roundtrip/*.ds` - Roundtrip stability tests
- `fixtures/formatter/smoke/**/input.*` - Formatter smoke cases

## Running

Run these commands from `language/`.

```bash
# Run all formatter tests
just test-formatter

# Run specific tests
cargo test --release --test formatter -- spacing     # tests matching "spacing"
cargo test --release --test formatter -- roundtrip   # only roundtrip tests
cargo test --release --test formatter -- transform   # only transform tests
cargo test --release --test formatter -- smoke       # only smoke tests
just test-formatter-conformance    # external suite conformance harness
```

## Fixture Sync

Use the oxfmt sync helper to refresh transform expected blocks from the external baseline.

```bash
python language/test/src/formatter/scripts/sync_transform_fixtures_with_oxfmt.py --dry-run
python language/test/src/formatter/scripts/sync_transform_fixtures_with_oxfmt.py
```
