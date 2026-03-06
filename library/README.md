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
`just library/quick` currently reports that dedicated library tests are not defined yet.

```sh
just library/format
just library/format-check
just library/check
just library/test
just library/quick
just library/full
```
