## Emit Tests

Emit tests verify emitted output against checked-in snapshots.

### Fixtures
Each test is a directory under `fixtures/emit/<name>/` containing:
- `destack.json`: target definitions
- `src/`: input sources
- `dist/<target>/`: expected output snapshots

The runner emits into `dist-actual/` and compares it to `dist/`.

### Running
```bash
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
cargo test -p destack_test --test emit -- hello-world
```

