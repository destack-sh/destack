## Smoke tests

Smoke tests ensure the parser and compiler don't crash on valid input and don't emit unexpected diagnostics.

### Fixtures
- `fixtures/smoke/parser/`: parser smoke tests (one file per test)
- `fixtures/smoke/compiler/`: compiler smoke tests (one file per test)

### Running
```bash
cargo test -p destack_test --test smoke
cargo test -p destack_test --test smoke -- --parser
cargo test -p destack_test --test smoke -- --compiler
cargo test -p destack_test --test smoke -- --list
```

