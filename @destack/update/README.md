Update installed Destack distributions with TUF verification.

## Usage

```ts
import { Updater } from "@destack/update";

using updater = await Updater.open({
    directory: installationDirectory,
    repository: new URL("https://download.destack.sh/"),
    root: trustedRoot,
    target,
    application: applicationPath, // macOS
});

const update = await updater.check();
if (update) {
    const download = await update.download({ signal, onProgress });
    const staged = await updater.stage(download);
}
```

## Restart

An update stays staged across sessions, and activates online once its processes have stopped.

```ts
using updater = await Updater.open(options);
const staged = await updater.staged();
if (staged) {
    // the CLI stops the desktop and the daemon first, and restarts them once activation succeeds
    const installed = await updater.activate(staged); // refreshes the signed metadata
}
```
