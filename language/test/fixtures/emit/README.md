# Emit Test Fixtures

Emit tests verify emitted output against checked-in snapshots.

## Structure

Each fixture lives under `fixtures/emit/<family>/<mode>/<scenario>/`.

Each fixture contains:
- `destack.json`: target definitions
- `src/`: input sources
- `dist/`: expected output snapshots
- `diagnostics.txt`: exact expected diagnostics for failure scenarios

The runner emits into `target/destack-test/emit/<case>/dist/` and compares it to `dist/`.
When `--update-snapshots` is set, the runner refreshes `dist/` from that transient output tree.
Failed runs preserve the transient output tree for inspection and print its path.
When `diagnostics.txt` exists, the runner compares exact diagnostics instead of `dist/`.

The fixture taxonomy should scale as:
- family: `script`, `html`, `wasm`, `native`
- mode: `single-file`, `preserve-modules`, `chunked`, `executable`, `shared-library`
- scenario: `static-import`, `external-package`, `entry-root`, `dynamic-import`, `manual-chunk-name`, `reject-dynamic-import`

Examples:
- `script/single-file/static-import`
- `script/single-file/external-package`
- `script/preserve-modules/banner-footer`
- `script/preserve-modules/include-roots`
- `script/chunked/dynamic-import`
- `script/chunked/named-output-files`
- `script/chunked/manual-chunk-name`
- `html/document-entry`
- `html/document-entry-public-path`

Use compiler-local linker tests for planner and rewrite logic.
Use emit fixtures for full target products, exact output trees, manifests, and maps.

Each fixture should represent one real product scenario.
Do not hide multiple unrelated linker behaviors in one oversized snapshot.
Do not introduce synthetic path segments like `ok` or `error` when the scenario itself has a clearer name.

The emit fixture assertions should remain exact.
Prefer full file content and full directory tree comparison over substring checks.

## Running

```bash
just test-emit
just test-emit-update
cargo test -p destack_test --test emit
cargo test -p destack_test --test emit -- --list
```
