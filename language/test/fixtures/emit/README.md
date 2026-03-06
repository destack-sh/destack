# Emit Test Fixtures

Emit tests verify emitted output against checked-in snapshots.

## Structure

Each test is a directory under `fixtures/emit/<name>/` containing:
- `dsconfig.json`: target definitions
- `src/`: input sources
- `dist/<target>/`: expected output snapshots

The runner emits into `dist-actual/` and compares it to `dist/`.

## Running

```bash
just test-emit
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
```
