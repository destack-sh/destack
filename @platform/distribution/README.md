# @platform/distribution

Ship Destack: build, sign and publish the desktop, the CLI and the machine as one distribution, with the first-party releases it bundles.

## Overview

`@platform/distribution` is Destack's release pipeline: distributions per target, a TUF update feed, and the first-party releases the platform publishes.

A release builds and ships:

- Distribution: the desktop, the CLI and the machine for one target, as an archive and an installer.
- Feed: the TUF repository at `download.destack.sh`, its root signed by two of three YubiKeys.
- `PlatformBuild`: the checkout's first-party releases, which a deployment ships and publishes into the platform accounts' forges.
- `Profile`: what a build is, `release` or `local`, with its command, identity and default universe.

Build and pack a distribution, and release main:

```sh
bun run build
bun run pack
bun run release
```

## Reference

### Recipes

| Recipe          | What it does                                               |
| --------------- | ---------------------------------------------------------- |
| `build`, `pack` | builds the distribution, and packs archives and installers |
| `installer`     | builds an installer                                        |
| `release`       | releases main as the month's next `vYYYY.M.N`              |

The release workflow releases main once a day.
`bun run src/main.ts update expiry` fails once a feed role nears expiry.
[RUNBOOK.md](RUNBOOK.md) covers publication, keys and their custody, and the Apple setup.

## Examples

### Install from the feed

The `install` script installs the Linux archive, and the macOS DMG's app registers the CLI and the machine on first launch.

```sh
curl -fsSL https://download.destack.sh/install | sh
```

### Name a build's profile

`Profile` gives a build its identity and the universe it signs in at by default.

```ts
import { Profile } from "@destack/update/release";

// read the release's title and the local build's command
export const title = new Profile("release").title;
export const command = new Profile("local").command;
```

## Comparison

### GitHub Releases

Each release is tagged and recorded as a GitHub Release, attested with build provenance.

| GitHub Releases | `@platform/distribution` | Difference                                                        |
| --------------- | ------------------------ | ----------------------------------------------------------------- |
| release assets  | the TUF feed             | One release ships every program, each file stored once by digest. |
| signed tags     | the feed's root          | Signed by two of three YubiKeys, its online keys renewed daily.   |
