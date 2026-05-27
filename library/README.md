# Library

The Destack standard library.
Integrated packages for building full-stack applications with Destack.

## Projects

| Project | Status | Summary |
|---------|--------|---------|
| [`schema`](schema/README.md) | Experimental | Shared schemas for cross-package contracts |
| [`test`](test/README.md) | Experimental | Library testing utilities and check support |

## Commands

Run these commands from the repository root.
`just library/check-quick` currently reports that dedicated library tests are not defined yet.

```sh
just library/format
just library/format-check
just library/lint
just library/test
just library/check-quick
just library/check-full
```
