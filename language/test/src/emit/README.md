## Emit Tests

Emit tests verify emitted output against checked-in snapshots.

### Fixtures

Each test is a directory under `fixtures/emit/<target>/<family>/<scenario>/` containing:

- `destack.json`: target definitions
- `src/`: input sources
- `dist/`: expected output snapshots for successful emission

Emit fixtures are positive output cases.
If a fixture currently produces diagnostics, the test should fail until the feature works.

### Running

```bash
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
cargo test -p destack_test --test emit -- script/form
cargo test -p destack_test --test emit -- --update-snapshots html/loaders
```
