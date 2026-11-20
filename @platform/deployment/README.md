# @platform/deployment

Deploy the platform Destack operates: its universe and cloud machines on Cloudflare, or the local universe on this computer.

## Overview

`@platform/deployment` deploys the platform for one universe, production on Cloudflare or a local one on loopback.

A deployment declares and runs:

- `Deployment`: the platform for one universe, from `Deployment.cloudflare` or `Deployment.local`.
- `Cloudflare`: applies a Cloudflare deployment's roots, and releases its Workers.
- `LocalUniverse`: serves a local universe at `destack.localhost`, its cloud machines and this computer's machine, on Bun or workerd.
- `Root`: one OpenTofu root with encrypted remote state: `access`, `global`, `universe` or `residency`.
- `Custody`: each Worker's root keys, encrypted under an AWS KMS key, in `custody/<universe>.json`.

Review, deploy and publish production, or serve a local universe:

```sh
bun run diff production universe
bun run deploy production
bun run src/main.ts platform publish production
just local serve
```

## Reference

### Recipes

| Recipe                           | What it does                                               |
| -------------------------------- | ---------------------------------------------------------- |
| `diff <universe> <root>`         | shows a root's planned changes                             |
| `deploy <universe>`              | deploys the platform for a universe                        |
| `custody seal <universe>`        | encrypts the root keys a universe's record lacks under KMS |
| `just local`, `just local reset` | runs a local universe, or resets its state                 |

[RUNBOOK.md](RUNBOOK.md) covers the roots, the Workers, key custody and the local universe.
