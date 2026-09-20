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

Staging persists across sessions. Activation refreshes signed metadata and requires network access.
Stop affected processes before activation; restart them after activation succeeds.

```ts
using updater = await Updater.open(options);
const staged = await updater.staged();
if (staged) {
    const installed = await updater.activate(staged);
}
```

The CLI coordinates desktop shutdown, daemon shutdown, activation, and restart.
