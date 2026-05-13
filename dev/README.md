# Development

Internal engineering automation for the Destack monorepo.

## Projects

| Project                  | Summary                                                                                            |
| ------------------------ | -------------------------------------------------------------------------------------------------- |
| [`ci`](ci)               | Repository policy checks, workflow validation, release automation, and project metadata validation |
| [`toolchain`](toolchain) | Runtime host setup, target tooling, and local environment support                                  |
|                          |                                                                                                    |

## Commands

Run these from the repository root.

```sh
just check-hygiene
just check-workflow-policy
just install-hygiene-toolchain
just language/install-toolchain
just bridge/install-toolchain
```
