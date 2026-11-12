# @destack/update

Update installed Destack distributions from a signed release feed.

## Overview

`@destack/update` is a TUF client and publisher for Destack's releases: update sessions, file manifests, and signed feeds.

Computers update their installation:

- `Distribution`: this computer's installation and the feed it follows.
- `Updater`: one locked update session.
- `Update`: a newer release, ready to download.
- `Release`: a version for one target triple.
- `Manifest`: a release's files by digest.

Devices:

- `distribution`: a device's installed and staged versions.

Publishers:

- `SigningKey`: a TUF signing key, generated or imported from PEM.
- `TrustedRoot`: the root metadata, signed by two of three root keys.
- `SignedRepository`: the feed on disk.
- `UpdateError`: the reasons an update or publication fails, checked with `UpdateError.is`.

Check for a newer release, stage it, and activate it:

```ts
import type { Distribution } from "@destack/update";

/** Update a distribution to the feed's newest release, when one is newer. */
export async function updateNow(distribution: Distribution) {
    // check the feed in a locked session
    await using updater = await distribution.open();
    const update = await updater.check();
    if (update === undefined) {
        return undefined;
    }

    // stage the release, then activate it
    const staged = await updater.stage(await update.download());

    return await updater.activate(staged);
}
```

## Reference

### Distribution

`new Distribution(options)` describes an installation.
A development build of the `local` stage refuses every update.

| Option         | What it is                                                                            |
| -------------- | ------------------------------------------------------------------------------------- |
| `home`         | the directory the installation keeps its releases in                                  |
| `repository`   | the feed's URL, such as `https://download.destack.sh/`                                |
| `root`         | the root metadata the feed is trusted from, `Distribution.root()` for the bundled one |
| `running`      | the `Release` running now                                                             |
| `executable`   | the running command's path                                                            |
| `applications` | the directory a Mac application is installed in                                       |
| `stage`        | the `Stage`: `production` or `local`, with its feed, universe and command             |
| `environment`  | the files, processes and host the update runs on                                      |

| Verb                               | What it does                                                    |
| ---------------------------------- | --------------------------------------------------------------- |
| `open()`                           | opens a locked `Updater`, refusing an installation that changed |
| `stage(source)`                    | downloads and stages the newest release when it is newer        |
| `installed()`                      | reads the installed release                                     |
| `staged()`                         | reads the release staged for activation, if any                 |
| `register(record)`, `record()`     | writes or reads how the distribution was installed              |
| `link(options)`, `unlink(options)` | adds the command to the user's shell and menus, or removes it   |
| `toolchain()`                      | reads the toolchain the release bundles                         |

### Updater

An `Updater` holds the installation lock until disposed.

| Verb                                              | What it does                                                                                         |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `check()`                                         | answers an `Update` for a newer authenticated release, or `undefined`                                |
| `stage(download)`                                 | verifies a download, running its `destack --version --json`, and stages it beside the active release |
| `activate(staged)`                                | refreshes trust, then makes a staged release the installed one                                       |
| `current()`                                       | reads the installed release                                                                          |
| `staged()`                                        | reads the staged release                                                                             |
| `update.download({ source, signal, onProgress })` | downloads a release's files, copying the ones the installation or `source` has                       |

### Release

A `Release` is `{ version, target }`, and `compare` orders releases by version.
`Release.target(platform, architecture)` reads a target from Node's names, such as `("darwin", "arm64")`.

| Target                      | System                 |
| --------------------------- | ---------------------- |
| `aarch64-apple-darwin`      | macOS on Apple silicon |
| `x86_64-apple-darwin`       | macOS on Intel         |
| `x86_64-pc-windows-msvc`    | Windows                |
| `aarch64-unknown-linux-gnu` | Linux on ARM           |
| `x86_64-unknown-linux-gnu`  | Linux on x86           |

### Manifest

A manifest is JSON of `files` (`path`, `digest`, `size`, `mode`) and `links` (`path`, `target`).
`checkout(directory, { blobs, sources, signal, onProgress })` copies the files it has locally, and fetches the rest.

### Publishing

`@destack/update/publish` writes the feed's TUF metadata: a root signed by two of three offline keys, and one online key per other role.
Targets last 365 days, and snapshots and timestamps 14.

| Verb                                                                                   | What it does                                                   |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `SigningKey.generate(scheme, crypto)`, `SigningKey.import(pem, crypto)`                | creates or reads a signing key                                 |
| `key.sign(metadata)`                                                                   | adds the key's signature to metadata                           |
| `TrustedRoot.create(version, roots, keys, expires, environment)`                       | creates an unsigned root over the public keys                  |
| `TrustedRoot.verify`, `authenticate`, `read`                                           | checks a root or a rotation against both quorums, or reads one |
| `SignedRepository.create(directory, revision, root, keys, distributions, environment)` | publishes releases, toolchains and installers                  |
| `SignedRepository.renew(directory, revision, root, keys, targets, environment)`        | signs the existing targets again before they lapse             |
| `SignedRepository.read`, `write`                                                       | reads and authenticates the feed, or writes new targets        |

## Examples

### Show download progress

`download` reports the bytes received out of the signed total.

```ts
import type { Updater } from "@destack/update";

/** Download the newest release, logging its progress, and stage it. */
export async function stageWithProgress(updater: Updater, signal: AbortSignal) {
    // check for a newer release
    const update = await updater.check();
    if (update === undefined) {
        return undefined;
    }

    // download it with progress, then stage it
    const download = await update.download({
        signal,
        onProgress: ({ received, total }) => console.log(`${received} of ${total} bytes`),
    });

    return await updater.stage(download);
}
```

### Sign a feed's root

`TrustedRoot.create` builds a root over the online keys, signed by two of three root keys.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { SigningKey, TrustedRoot } from "@destack/update/publish";

// generate three root keys and one key per online role
const environment = BunEnvironment.system;
const generate = () => SigningKey.generate("ed25519", environment.crypto);
const rootKeys = [await generate(), await generate(), await generate()];
const [targets, snapshot, timestamp] = [await generate(), await generate(), await generate()];

// create the root over the public keys, lasting a year
const root = TrustedRoot.create(
    1,
    rootKeys.map((key) => key.public),
    { targets: targets.public, snapshot: snapshot.public, timestamp: timestamp.public },
    "2027-11-12T00:00:00Z",
    environment,
);

// sign it with two of the three root keys
for (const key of rootKeys.slice(0, 2)) {
    await key.sign(root);
}
```

### Update from the terminal

`destack self update` stages the newest release and restarts into it, and `--stage` stops after staging.

```sh
destack self update --stage
destack self update
```

## Comparison

### The Update Framework

Roots, targets, snapshots, timestamps and their expiry follow TUF, verified by `tuf-js`.
An update downloads only the files that changed.

| TUF                                 | `@destack/update`             | Difference                                                                    |
| ----------------------------------- | ----------------------------- | ----------------------------------------------------------------------------- |
| `root.json`                         | `TrustedRoot`                 | Two of three root keys, and one key per online role.                          |
| a target file                       | a release's signed `Manifest` | The release's files are blobs by digest, copied from installed ones.          |
| `Updater.refresh`, `downloadTarget` | `check`, `download`           | The same, under an installation lock.                                         |
| none                                | `stage`, `activate`           | A staged release is verified, and trust refreshed again, before it activates. |
| key thresholds, expiry              | the same                      | Targets last 365 days, snapshots and timestamps 14.                           |

### Sparkle

Daily checks, background staging and restart prompts follow Sparkle.

| Sparkle                 | `@destack/update`           | Difference                                                |
| ----------------------- | --------------------------- | --------------------------------------------------------- |
| the appcast             | the TUF feed                | Signed metadata with key rotation instead of an RSS feed. |
| "Check for Updates…"    | `check-for-updates`         | The same, as a command of the device's `distribution`.    |
| "Install and Relaunch"  | `restart-to-update`         | Restarts the machine and relaunches the desktop.          |
| automatic checks, daily | the machine's daily `stage` | The same.                                                 |
