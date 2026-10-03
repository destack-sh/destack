Update installed Destack distributions with TUF verification.

## Updates

An `Updater` locks an installation, checks the TUF repository for a newer release of its target, and downloads and stages it.

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

`Release.target()` names the running target, one of `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-msvc`, `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-gnu`.

## Restart

A staged update stays staged across sessions and activates online once its processes have stopped.

```ts
await using updater = await Updater.open(options);
const staged = await updater.staged();
if (staged !== undefined) {
    // the CLI stops the desktop and the daemon first, and restarts them once activation succeeds
    const installed = await updater.activate(staged); // refreshes the signed metadata
}
```

## Publishing

`@destack/update/publish` writes the signed TUF repository clients read: a two-of-three root, and one online key each for targets, snapshot and timestamp.

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
