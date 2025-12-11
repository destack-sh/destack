## Formatter tests

Formatter tests are roundtrip checks: formatting a file should produce the same text.

### Fixtures
- `fixtures/formatter/*.ds`
- `fixtures/formatter/*.d.ds`

### Running
```bash
cargo test -p destack_test --test formatter
cargo test -p destack_test --test formatter -- --list
cargo test -p destack_test --test formatter -- parser-0017
```

