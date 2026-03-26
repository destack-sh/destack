## Emit Tests

Emit tests verify emitted output against checked-in snapshots.

### Fixtures

Each test is a directory under `fixtures/emit/<family>/<mode>/<scenario>/` containing:

- `destack.json`: target definitions
- `src/`: input sources
- `dist/`: expected output snapshots

Emit fixtures are success fixtures only.
Compiler and linker diagnostics belong in compiler-local tests, not `diagnostics.txt` snapshots here.

### Running

```bash
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
cargo test -p destack_test --test emit -- script/single-file
cargo test -p destack_test --test emit -- --update-snapshots script/single-file
```
