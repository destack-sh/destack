# @destack/desktop

Open Destack apps in desktop windows, with banners, a badge, a tray and a command palette.

## Overview

The desktop runs beside this computer's machine and shows the signed-in person's apps through a native shell.

- **Desktop**: the one desktop of this computer; `start` runs it until the shell closes, `signIn`, `signOut` and `openPalette`.
- **Person**: the person signed in on this computer, with their windows, banners, badge, tray and shortcut.
- **Window**: a `window` object of this computer; `open`, `focus`, `place` and `close`, which the desktop shows as native windows.
- **Link**: the `destack://` deep link of a view or an object; `parse` and `format`.
- **Banner**: a message to this computer whose notification is unread, withdrawn once read.

```ts
import { Desktop } from "@destack/desktop";
import { BunEnvironment } from "@destack/environment/bun";

await Desktop.start(BunEnvironment.system);
```

## Examples

### Run the desktop in development

`bun run dev` runs the native shell with the development channel's identity and icons.

```sh
cd @destack/desktop
bun run dev
```

### Open a window

A window is an object like any other, so any program opens one by creating it.

```ts
import { window } from "@destack/desktop";
import type { ObjectClient } from "@destack/object/client";

async (objects: ObjectClient<{ window: typeof window }>) => {
    const link = "destack://notes--personal--ada/home";
    const opened = await objects.mutate(window).open({ link }).confirmed;
};
```

### Read a deep link

`Link` reads and writes the deep links of a stage's desktop scheme.

```ts
import { Link } from "@destack/desktop/link";

const links = new Link("destack");
const address = links.parse("destack://notes--personal--ada/home/drafts");
```

### Launch the installed desktop

`Application` launches the installed desktop from another program, such as the CLI's `open`.

```ts
import { Application } from "@destack/desktop";
import { BunEnvironment } from "@destack/environment/bun";

async (home: string, application: string) => {
    const desktop = new Application(home, BunEnvironment.system);
    await desktop.launch(application, "destack://notes--personal--ada/home");
};
```

## Comparison

### Electron

Windows, the tray, the badge, notifications and the application menu follow Electron's; three things differ.

- A native Tauri shell draws them, driven by the desktop over a pipe, so the desktop itself runs on Bun beside the machine.
- Windows are objects of this computer, so any program opens, focuses or closes one by calling it.
- Menus, banners and the palette come from the commands and notifications the person's installed packages declare.
