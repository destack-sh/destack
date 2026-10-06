# @destack/update

Update installed Destack distributions with TUF verification.

## Updates

`Updater.open` locks an installation, `check` looks up a newer release of its target in the TUF repository, and `stage` extracts and checks a downloaded release.

```ts
import { Release, Updater } from "@destack/update";

await using updater = await Updater.open({
    directory: installationDirectory,
    repository: new URL("https://download.destack.sh/stable/"),
    channel: "stable", // refuse releases of another channel
    root: trustedRoot,
    target: Release.target(process.platform, process.arch),
    application: applicationPath, // macOS only, with its bundle identifier
    applicationIdentifier: "app.destack.desktop",
});

const update = await updater.check();
if (update !== undefined) {
    const download = await update.download({ signal, onProgress });
    const staged = await updater.stage(download);
}
```

## Distributions

`Distribution` is this machine's installed distribution: the channel it follows, where it lives and the release running, which stages and activates through a locked `Updater`.

```ts
import { Distribution, Release } from "@destack/update";

const distribution = new Distribution({
    channel: "stable",
    home: "/Users/ada/.destack",
    repository: new URL("https://download.destack.sh/stable/"),
    root: Distribution.root("stable"), // the root metadata each release bundles
    running: new Release("2026.10.1", "aarch64-apple-darwin"),
    executable: process.execPath,
    applications: "/Users/ada/Applications",
    applicationIdentifier: "app.destack.desktop",
    title: "Destack",
});
const staged = await distribution.stage(); // the newer release, verified and staged, or none
await using updater = await distribution.open(); // activate under the installation lock
```

### Objects

`distribution` keeps the machine's distribution as an object the daemon hosts: its channel, the installed release, the staged one and whether a restart runs.

```ts
await client.distribution.stage({ machineId, requestId, id }); // { installed: "2026.10.1", staged: "2026.10.2", status: "staged", … }
await client.distribution.activate({ machineId, requestId, id }); // status "activating", then the daemon hands the restart over
```

### Service

`serveDistribution` stages through the host's `Distribution` and hands activation to its `activate`, and `DistributionController` records the distribution once and stages daily.

```ts
const objects = {
    distribution: serveDistribution({ distribution, activate: () => spawnActivation() }),
};
const controllers = [new DistributionController(server, machineId, { distribution, activate })];
```

## Targets

`Release.target(platform, architecture)` returns the target of a platform and architecture as Node names them, one of these five.

```text
aarch64-apple-darwin
x86_64-apple-darwin
x86_64-pc-windows-msvc
aarch64-unknown-linux-gnu
x86_64-unknown-linux-gnu
```

## Restart

`staged` returns a release staged in an earlier session, and `activate` verifies it against fresh metadata and installs it after the caller stops the running processes.

```ts
await using updater = await Updater.open(options);
const staged = await updater.staged();
if (staged !== undefined) {
    // `destack self activate` stops the daemon, which the desktop quits with, and starts them again once activation succeeds
    const installed = await updater.activate(staged); // refreshes the signed metadata
}
```

## Publishing

`SignedRepository.create` writes a signed TUF repository whose root needs two of three root keys and whose targets, snapshot and timestamp roles each have one online key.

```ts
import { SignedRepository, SigningKey, TrustedRoot } from "@destack/update/publish";

const keys = {
    targets: SigningKey.generate(),
    snapshot: SigningKey.generate(),
    timestamp: SigningKey.generate(),
};
const online = {
    targets: keys.targets.public,
    snapshot: keys.snapshot.public,
    timestamp: keys.timestamp.public,
};
const root = TrustedRoot.create(1, [first.public, second.public, third.public], online, expires);
root.sign((bytes) => first.sign(bytes)); // the ceremony signs with two of the three root keys
root.sign((bytes) => second.sign(bytes));
await SignedRepository.create(directory, revision, root, keys, [
    { target, version, commit, archive },
]);
await SignedRepository.renew(directory, revision, root, keys, targetBytes); // fresh snapshot and timestamp
```

## Lifetimes

`LIFETIME_DAYS` keeps targets valid for a year, and snapshot and timestamp for two weeks.

```text
targets     365 days   signed with each release
snapshot     14 days   re-signed by every publication and renewal
timestamp    14 days   re-signed by every publication and renewal
```

## Errors

A failed check, download, activation or installation throws an `UpdateError` with a code naming the failed step, and `toServiceError` names the service error its caller receives.

```ts
import { UpdateError } from "@destack/update/error";

new UpdateError("BUSY", "another process is updating this installation").toServiceError(); // { code: "CONFLICT", … }
```

## Tests

`UpdateFixture` serves a signed repository and an isolated installation over loopback, and `COMMIT` is the commit its releases claim.

```ts
import { createRootKey, UpdateFixture } from "@destack/update/test";

const rootKeys = [createRootKey(), createRootKey(), createRootKey()];
await using fixture = await UpdateFixture.open(compiled, rootKeys, "2026.9.1"); // publishes the compiled distribution
const updater = await Updater.open(fixture.options);
```
