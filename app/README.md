# App

First-party applications and programmer tools.

## Components

| Component | Description | Link |
|-----------|-------------|------|
| `cli` | Command-line interface (`destack`, `ds`, `dsc`, `dsx`). | [cli/README.md](cli/README.md) |

## Commands

Run these commands from the repository root.
Use `just app/test` as an alias for `just app/test-quick`.

```sh
just app/format
just app/check
just app/build
just app/test
just app/test-quick
just app/test-ci
just app/test-nightly
just app/test-release
just app/generate-schema
just app/publish --dry-run
```
