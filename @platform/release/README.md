# @platform/release

Ship Destack: build, sign and publish the desktop, the CLI and the machine as one distribution, and deploy the operated platform.

## Overview

`@platform/release` is Destack's release pipeline: distributions per target, a TUF update feed, and OpenTofu deployments of the platform.

A release builds and ships:

- Distribution: the desktop, the CLI and the machine for one target, as an archive and an installer.
- Feed: the TUF repository at `download.destack.sh`, its root signed by two of three YubiKeys.
- `Universe`: the universe a build belongs to, `production` or `local`, with its origin.

A release deploys:

- Deployment: the platform deployed for a universe, with its residencies and cloud machines.
- Roots: OpenTofu roots with encrypted remote state: `access`, `global`, `universe` and `residency`.
- Custody: each Worker's root keys, encrypted under an AWS KMS key, in `custody/<universe>.json`.

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
| `diff <universe> <root>`         | shows a root's planned changes                             |
| `deploy <universe>`              | deploys the platform for a universe                        |
| `release`                        | releases main as the month's next `vYYYY.M.N`              |
| `custody seal <universe>`        | encrypts the root keys a universe's record lacks under KMS |
| `just local`, `just local reset` | runs a local universe, or resets its state                 |

The release workflow releases main once a day.
`bun run src/main.ts update expiry` fails once a feed role nears expiry.

## Examples

### Install from the feed

The `install` script installs the Linux archive, and the macOS DMG's app registers the CLI and the machine on first launch.

```sh
curl -fsSL https://download.destack.sh/install | sh
```

### Name a build's universe

`Universe` gives a build its identity and the origin people sign in at.

```ts
import { Universe } from "@destack/update/release";

// read the production build's title and the local universe's command
export const title = Universe.of("production").title;
export const command = Universe.of("local").command;
```

## Comparison

### GitHub Releases

Each release is tagged and recorded as a GitHub Release, attested with build provenance.

| GitHub Releases | `@platform/release` | Difference                                                        |
| --------------- | ------------------- | ----------------------------------------------------------------- |
| release assets  | the TUF feed        | One release ships every program, each file stored once by digest. |
| signed tags     | the feed's root     | Signed by two of three YubiKeys, its online keys renewed daily.   |
| none            | `deploy`            | The same release deploys the platform through OpenTofu.           |
