# @destack/update

Update installed Destack distributions from a signed release feed.

## Overview

A distribution follows a TUF repository of releases, and its updater checks, downloads, stages and activates each release under an installation lock.

- **Distribution**: this device's installed Destack: where it lives, the feed it follows and the release running; `stage` fetches a newer release.
- **Updater**: a TUF client of one feed; `check`, `stage`, `staged` and `activate`.
- **Release**: a version for one target, named by Rust's target triple; `Release.target` names the host's.
- **Manifest**: a release's files by digest, which a checkout copies from what is installed already.
- **SignedRepository**: the feed a publisher writes, its root signed by two of three keys.

```ts
import type { Distribution } from "@destack/update";

async (distribution: Distribution) => {
    const updater = await distribution.open();
    const update = await updater.check();
    if (update !== undefined) {
        const staged = await updater.stage(await update.download());
        await updater.activate(staged);
    }
};
```

## Examples

### Update from the terminal

`destack self update` stages the newest release, activates it, restarts the machine and relaunches the desktop.

```sh
destack self update --stage
destack self update
```

### Name a release's target

`Release.target` names an operating system and architecture by Rust's target triple.

```ts
import { Release } from "@destack/update";

Release.target("darwin", "arm64"); // "aarch64-apple-darwin"
```

### Sign a feed's root

`TrustedRoot.create` builds a root over the online keys, which two of three root keys then sign.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { SigningKey, TrustedRoot } from "@destack/update/publish";

const environment = BunEnvironment.system;
const generate = () => SigningKey.generate("ed25519", environment.crypto);
const rootKeys = [await generate(), await generate(), await generate()];
const [targets, snapshot, timestamp] = [await generate(), await generate(), await generate()];
const root = TrustedRoot.create(
    1,
    rootKeys.map((key) => key.public),
    { targets: targets.public, snapshot: snapshot.public, timestamp: timestamp.public },
    "2027-11-12T00:00:00Z",
    environment,
);
for (const key of rootKeys.slice(0, 2)) {
    await key.sign(root);
}
```

## Comparison

### The Update Framework

Roots, targets, snapshots, timestamps and their expiry follow TUF through `tuf-js`; three things differ.

- A release is a manifest of files by digest, so an update downloads only the files that changed.
- A staged release is verified again against fresh metadata before it activates.
- Updating runs under an installation lock, so two programs never install at once.
