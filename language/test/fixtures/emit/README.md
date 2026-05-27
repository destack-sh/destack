# Emit Test Fixtures

Emit tests verify emitted output against checked-in snapshots.

## Structure

Each fixture lives under `fixtures/emit/<target>/<family>/<scenario>/`.
The target roots are `html` and `script`.

Each fixture contains:

- `destack.json`: target definitions
- `src/`: input sources
- `dist/`: expected output snapshots

The runner emits into `target/destack-test/emit/<case>/dist/` and compares that tree to `dist/`.

## Running

Use the emit runner directly when you want narrow family filters.

```bash
just test-emit
just test-emit-update
cargo run -p destack_test --bin emit -- html/loaders --update-snapshots
cargo run -p destack_test --bin emit -- script/chunking
```
