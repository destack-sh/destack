# Emit Test Fixtures

Emit tests verify emitted output against checked-in snapshots.

## Structure

Each fixture lives under `fixtures/emit/<family>/<mode>/<scenario>/`.

Each fixture contains:

- `destack.json`: target definitions
- `src/`: input sources
- `dist/`: expected output snapshots

The runner emits into `target/destack-test/emit/<case>/dist/` and compares it to `dist/`.

## Running

```bash
just test-emit
just test-emit-update
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
```
