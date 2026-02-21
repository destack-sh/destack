# Library

The Destack standard library.
Integrated packages for building full-stack applications with Destack.

## Packages

| Package | Description | Link |
|---------|-------------|------|
| `entity` | Core entity system, events, and paths | [entity/README.md](entity/README.md) |
| `test` | Testing utilities | [test/README.md](test/README.md) |
| `schema` | Shared schemas for cross-package contracts | [schema/README.md](schema/README.md) |

## Commands

Run these commands from the repository root.
Use `just library/test` as an alias for `just library/test-quick`.
`just library/test-quick` currently reports that dedicated library tests are not defined yet.

```sh
just library/format
just library/check
just library/test
just library/test-quick
just library/test-ci
just library/test-nightly
just library/test-release
```
