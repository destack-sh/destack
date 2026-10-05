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
    // the CLI stops the desktop and the daemon first, and restarts them once activation succeeds
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
