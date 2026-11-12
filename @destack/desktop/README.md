# @destack/desktop

Open Destack apps in desktop windows, with banners, a badge, a tray and a command palette.

## Overview

`@destack/desktop` is the desktop app beside a computer's machine: native windows, banners, a badge, a tray, menus and a command palette over a Tauri shell.

The desktop runs on each computer:

- `Desktop`: this computer's running desktop.
- `Person`: the person signed in, with their windows and icons.
- `Shell`: the native shell, driven by commands and answering with events.

People see their apps and notifications:

- `window`: a native window showing a view, an object or the palette.
- `Banner`: a system banner for an unread notification.
- `Badge`, `Tray` and `Menu`: the app icon's unread count, the tray's status, and the menus.
- `Shortcut`: a global shortcut, such as the palette's.

Apps link to each other:

- `Link`: a `destack://` deep link to a view or an object.

Start the machine and run the desktop until its shell closes:

```ts
import { Desktop } from "@destack/desktop";
import { BunEnvironment } from "@destack/environment/bun";

// start the machine and run the desktop
await Desktop.start(BunEnvironment.system);
```

## Reference

### Desktop

`Desktop.start(environment)` starts this computer's machine and runs the desktop until the shell closes its pipe.

| Verb                    | What it does                                         |
| ----------------------- | ---------------------------------------------------- |
| `run(link)`             | signs in, opens a link, and answers each shell event |
| `signIn()`, `signOut()` | starts a browser sign-in, or signs the person out    |
| `switch()`              | signs the person out and another in                  |
| `openLink(url)`         | opens a link in a window, or the person's start link |
| `dispatch(event)`       | answers one shell event                              |

### Link

`Link.of(stage)` writes and reads the deep links of a stage's scheme, such as `destack://`.

| Verb                  | What it does                           |
| --------------------- | -------------------------------------- |
| `parse(href)`         | reads the view or object of a link     |
| `format(address)`     | writes the link to a view or an object |
| `Link.title(address)` | reads the name a link shows under      |

### Shell

The shell is a Tauri process on the host's private pipes, one JSON line per command or event.

| Command                                       | What it does                                |
| --------------------------------------------- | ------------------------------------------- |
| `window.open`, `window.focus`, `window.close` | opens, focuses or closes a window           |
| `banner.show`, `banner.withdraw`              | shows or withdraws a banner                 |
| `badge.set`                                   | shows or clears the icon's count            |
| `menu.set`, `menu.title`                      | fills a menu, and titles its built-in items |
| `tray.set`                                    | sets the tray's status text                 |
| `shortcut.register`, `shortcut.unregister`    | registers or releases a global shortcut     |
| `browser.open`                                | opens a location in the default browser     |
| `profile.remove`                              | removes a browser profile and its storage   |
| `autostart.set`                               | opens the desktop at login, or stops        |
| `application.quit`                            | closes every window and exits               |

| Event                                             | What happened                                 |
| ------------------------------------------------- | --------------------------------------------- |
| `window.closed`, `window.moved`, `window.focused` | a window closed, moved or came into focus     |
| `window.navigated`                                | a window navigated off its origin             |
| `banner.acted`, `banner.dismissed`                | a banner's action was chosen, or dismissed    |
| `link.opened`                                     | a link was opened, or a new window asked for  |
| `menu.chosen`, `shortcut.pressed`                 | a menu item was chosen, or a shortcut pressed |
| `application.reopened`                            | the desktop was launched again                |
| `shell.failed`                                    | a command or native step failed               |

## Comparison

### Tauri

The native shell is Tauri 2, with its tray, notification, global shortcut and autostart plugins.

| Tauri                       | `@destack/desktop`    | Difference                                                    |
| --------------------------- | --------------------- | ------------------------------------------------------------- |
| commands and events         | `Command` and `Event` | The host logic runs in Bun, and drives the shell over pipes.  |
| windows                     | `window` objects      | Windows are rows, so their state syncs and survives restarts. |
| `tauri-plugin-notification` | `Banner`              | Banners follow the person's inbox and withdraw once read.     |
