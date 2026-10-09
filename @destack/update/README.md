# @destack/update

`Updater` is a TUF client over `tuf-js` that checks, downloads, stages and activates a distribution's releases under an installation lock, `/publish` writes the signed TUF repository with a two-of-three root and online targets, snapshot and timestamp keys, and `Release.target` names targets by Rust's target triples.

```ts
await using updater = await Updater.open({ directory, repository, channel: "stable", root: trustedRoot, target }); // tuf-js's Updater
const update = await updater.check(); // refreshes root, timestamp, snapshot and targets metadata
const staged = await updater.stage(await update.download({ signal, onProgress })); // verified against the targets' hashes
await SignedRepository.create(directory, revision, root, keys, releases); // a TUF repository
Release.target("darwin", "arm64"); // "aarch64-apple-darwin"
```

## Distributions

`Distribution` is this machine's installed distribution: the channel it follows, where it lives and the release running.

```ts
import { Distribution, Release } from "@destack/update";

const distribution = new Distribution({
    channel: "stable",
    home: "/Users/ada/.destack",
    repository: new URL("https://download.destack.sh/stable/"),
    root: Distribution.root("stable"),
    running: new Release("2026.10.1", "aarch64-apple-darwin"),
    executable: process.execPath,
    applications: "/Users/ada/Applications",
    applicationIdentifier: "app.destack.desktop",
    title: "Destack",
});
const staged = await distribution.stage(); // the newer release, verified and staged, or none
```

## Restart

`staged` returns a release staged in an earlier session, and `activate` verifies it against fresh metadata and installs it after the caller stops the running processes.

```ts
const staged = await updater.staged();
if (staged !== undefined) {
    await updater.activate(staged);
}
```

## Publishing

`TrustedRoot.create` builds a root that two of three root keys sign, and `SignedRepository.renew` signs a fresh snapshot and timestamp.

```ts
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";

const root = TrustedRoot.create(1, [first.public, second.public, third.public], online, expires);
root.sign((bytes) => first.sign(bytes));
root.sign((bytes) => second.sign(bytes));
await SignedRepository.renew(directory, revision, root, keys, targetBytes);
LIFETIME_DAYS; // { targets: 365, snapshot: 14, timestamp: 14 }
```

## Objects

`distribution` keeps the machine's distribution as an object, which `serveDistributions` serves and `DistributionController` stages daily.

```ts
await client.distribution.stage({ machineId, requestId, id }); // { installed: "2026.10.1", staged: "2026.10.2", status: "staged", … }
await client.distribution.activate({ machineId, requestId, id });
```
