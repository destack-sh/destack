# @destack/update

Update installed Destack distributions with TUF verification.

## Updates

`Updater.open` locks an installation, `check` looks up a newer release of its target in the TUF repository, and `stage` extracts and checks a downloaded release.

```ts
import { Release, Updater } from "@destack/update";

await using updater = await Updater.open({
    directory: installationDirectory,
    repository: new URL("https://download.destack.sh/"),
    root: trustedRoot,
    target: Release.target(),
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

`Release.target()` returns the running target, one of these five.

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
await SignedRepository.create(directory, revision, root, keys, [{ target, version, archive }]);
await SignedRepository.renew(directory, revision, root, keys, targetBytes); // fresh snapshot and timestamp
```
