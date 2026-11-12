# @platform/release

Ship Destack: build, sign and publish the desktop, the CLI and the machine as one distribution, and deploy the operated platform.

## Overview

A release builds the distribution for every target, signs it, publishes it to the update feed, and deploys the platform's universe and cloud machines.

- **Distribution**: the desktop, the CLI and the machine built for one target, packed as an archive and an installer.
- **Feed**: the TUF repository at `download.destack.sh`, its root signed by two of three YubiKeys and its online keys renewed daily.
- **Deployment**: the platform at a stage, declaring its universe, its residencies and one cloud machine per residency.
- **Root**: an OpenTofu root with a separate encrypted remote state: `access`, `global`, `universe` and `residency`.
- **Custody**: each Worker's root keys, sealed by an AWS KMS key and recorded in `custody/<stage>.json`.
- **Stage**: a build's identity, `production`, `dev` or `local`, with its title, command, icons and device directory.

```sh
just @platform/release/build
just @platform/release/pack
just @platform/release/diff production universe
just @platform/release/deploy production
```

## Examples

### Release main now

`release` releases main as the month's next `vYYYY.M.N`, which the release workflow otherwise does once a day.

```sh
just @platform/release/release
```

### Run a local universe

`just local` starts the universe, a cloud machine per residency and the local stage's machine over a local database.

```sh
just local
just local reset
```

### Seal a stage's keys

`custody seal` seals the keys a stage's record lacks, each under its root's KMS key, once per key.

```sh
just @platform/release/custody seal production
```

### Check the feed's expiry

`update expiry` fails once a role's metadata is close to expiring.

```sh
DESTACK_STAGE=production bun run @platform/release/src/main.ts update expiry
```

### Name a build's stage

`Stage` gives a build its identity, which the build and the installed programs read.

```ts
import { Stage } from "@destack/update/release";

Stage.of("production").title; // "Destack"
Stage.of("local").command; // "destack-local"
```

### Install from the feed

The `install` script installs the Linux archive, and the macOS DMG's application registers the CLI and the machine on first launch.

```sh
curl -fsSL https://download.destack.sh/install | sh
```

## Comparison

### GitHub Releases

Each release is tagged and recorded as a GitHub Release, its archives and installers attested with build provenance; three things differ.

- The update feed is a TUF repository, and one release ships the desktop, the CLI and the machine together, each target's files stored once by digest.
- The feed's root is signed by two of three YubiKeys in a ceremony, and its online keys sign daily renewals.
- The same release deploys the platform's universe and cloud machines through OpenTofu roots.
