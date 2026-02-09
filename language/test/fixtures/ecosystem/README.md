# Ecosystem Fixtures

This directory stores manifest driven ecosystem fixtures for real package validation.
The canonical ecosystem status table is maintained in `language/test/src/ecosystem/README.md`.

## Structure

The fixture tree uses the following layout.

- `packages/`: package manifests with pinned repository refs.
- `patches/`: per-package overlay files copied into checkouts when needed.
- `known-failures.txt`: temporary known failures by case id.
- `ignored.txt`: intentionally skipped case ids.
- `checkouts/`: local clone checkouts, excluded from git.

## Running

Fetch package checkouts before running ecosystem tests.
Missing checkouts are also fetched automatically by the ecosystem runner.

```bash
just language/ecosystem-fetch
cargo test -p destack_test --test ecosystem
cargo test -p destack_test --test ecosystem -- --phase parse
cargo test -p destack_test --test ecosystem -- --all-phases
```

## Notes

Checkouts are intentionally mutable and may be refreshed with fetch commands.
Patch overlays are stamped and re-applied only when patch content changes.
Compiler phase entrypoints prefer package manifest entry fields before falling back to source sampling.
Known failures are skipped by default, so use `--include-known-failures` to force execution and print diagnostics.
