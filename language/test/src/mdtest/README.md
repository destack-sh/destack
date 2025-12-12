## Markdown Tests

Markdown tests are for specification testing.
They currently run type checking and assert expected diagnostics for small, focused examples.

### Fixtures
- `fixtures/mdtest/**/*.md`

The markdown format is documented in `fixtures/mdtest/README.md`.

### Expectations
- By default, each bullet is treated as an **exact** expected error message (case and whitespace normalized)
- Use `contains:<text>` to opt into explicit substring matching

### Running
```bash
cargo test -p destack_test --test mdtest
cargo test -p destack_test --test mdtest -- --list
cargo test -p destack_test --test mdtest -- variables
```

