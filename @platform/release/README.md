# @platform/release

Ship Destack: build, sign and publish the desktop, the CLI and the machine as one distribution, and deploy the operated platform.

## Overview

`@platform/release` is Destack's release pipeline: distributions per target, a TUF update feed, and OpenTofu deployments of the platform.

A release builds and ships:

- Distribution: the desktop, the CLI and the machine for one target, as an archive and an installer.
- Feed: the TUF repository at `download.destack.sh`, its root signed by two of three YubiKeys.
- `Stage`: a build's identity, `production`, `dev` or `local`.

A release deploys:

- Deployment: the platform at a stage, with its universe, residencies and cloud machines.
- Roots: OpenTofu roots with encrypted remote state: `access`, `global`, `universe` and `residency`.
- Custody: each Worker's root keys, encrypted under an AWS KMS key, in `custody/<stage>.json`.

Build, pack, review and deploy:

```sh
just @platform/release/build
just @platform/release/pack
just @platform/release/diff production universe
just @platform/release/deploy production
```

## Reference

### Recipes

| Recipe                           | What it does                                               |
| -------------------------------- | ---------------------------------------------------------- |
| `build`, `pack`                  | builds the distribution, and packs archives and installers |
| `diff <stage> <root>`            | shows a root's planned changes                             |
| `deploy <stage>`                 | deploys the platform at a stage                            |
| `release`                        | releases main as the month's next `vYYYY.M.N`              |
| `custody seal <stage>`           | encrypts the root keys a stage's record lacks under KMS    |
| `just local`, `just local reset` | runs a local universe, or resets its state                 |

The release workflow releases main once a day.
`bun run src/main.ts update expiry` fails once a feed role nears expiry.

## Examples

### Install from the feed

The `install` script installs the Linux archive, and the macOS DMG's app registers the CLI and the machine on first launch.

```sh
curl -fsSL https://download.destack.sh/install | sh
```

### Name a build's stage

`Stage` gives a build its identity.

```ts
import { Stage } from "@destack/update/release";

// read the production build's title and the local stage's command
export const title = Stage.of("production").title;
export const command = Stage.of("local").command;
```

## Comparison

### GitHub Releases

Each release is tagged and recorded as a GitHub Release, attested with build provenance.

| GitHub Releases | `@platform/release` | Difference                                                        |
| --------------- | ------------------- | ----------------------------------------------------------------- |
| release assets  | the TUF feed        | One release ships every program, each file stored once by digest. |
| signed tags     | the feed's root     | Signed by two of three YubiKeys, its online keys renewed daily.   |
| none            | `deploy`            | The same release deploys the platform through OpenTofu.           |
