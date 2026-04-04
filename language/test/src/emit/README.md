## Emit Tests

Emit tests verify emitted output against checked-in snapshots.

### Fixtures

Each test is a directory under `fixtures/emit/<emit>/<concern>/<scenario>/` containing:

- `destack.json`: target definitions
- `src/`: input sources
- `dist/`: expected output snapshots for successful emission

Emit fixtures are positive output cases.
If a fixture currently produces diagnostics, the test should fail until the feature works.

The top-level `emit` directory should match the actual target `emit` value, for example:

- `js`
- `html`

The `concern` directory should describe the main observable output behavior, for example:

- `asset`
- `style`
- `dependency`
- `entry`
- `chunking`
- `minify`
- `naming`
- `sourcemap`

### Running

```bash
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
cargo test -p destack_test --test emit -- js/style
cargo test -p destack_test --test emit -- --update-snapshots html/asset
```
