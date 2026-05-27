# Development

Internal engineering automation for the Destack monorepo.

## Projects

| Project | Summary |
|---------|---------|
| [`ci`](ci) | Hygiene toolchain setup and release automation |
| [`toolchain`](toolchain) | Runtime host setup, target tooling, and local environment support |

## Commands

Run these from the repository root.

```sh
just check-hygiene
just install-hygiene-toolchain
just language/install-toolchain
just bridge/install-toolchain
```
