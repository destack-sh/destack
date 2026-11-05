# @destack/desktop

The desktop shows this computer's windows, banners, badge, tray and command palette through a native shell, acting as the person signed in.

## Development

`bun run dev` runs the native shell with the development channel's identity and icons.

```sh
cd @destack/desktop
bun run dev
```

## Workload

`Desktop.start` starts the machine and runs the desktop until the shell closes its pipe, and `Application` launches the installed desktop from outside.

```ts
await Desktop.start(environment);
await new DesktopLock(native.home, environment).isHeld(); // whether a desktop runs
await new Application(home, environment).launch(application, "destack://notes--personal--florian/home");
```

## Person

Each native item is one noun named after its shell actions, and the signed-in `Person` has the windows, banners, badge, tray and shortcut showing what is theirs.

```ts
const person = await Person.open(desktop, session, machineId, accountId);
await person.answer({ action: "window.focused", id }); // the shell's window, dialog, banner and shortcut events
await person.badge.show(3); // badge.set { count: 3 }
await person.shortcut.register("alt+space"); // shortcut.register { id: "palette", shortcut: "alt+space" }
await person.stop(); // stop the follows, close the clients and flush the telemetry
await person.clear(); // close the windows, withdraw the banners, clear the badge, tray, shortcut and settings item
```

## Signing in

The desktop shows the one `Person` signed in on this computer once their sign-in enrolled its machine, following the device's logins and the machine's record, whichever program signed them in or out.

```ts
await desktop.signIn(); // start a browser sign-in unless someone is signed in; the follow shows them
await desktop.signOut(); // revoke this computer's client; the follow clears the person from the screen
Desktop.accountEntries({ status: "signedIn", login: { name: "Florian" } }, desktop); // "Florian", "Switch Account…", "Sign Out"
```

## Windows

A window is a `window` object of this computer, controlled like any controlled object: `Window.plan` turns its rows into steps, and the person shows, focuses, merges and removes their `Window`s by them.

```ts
await objects.mutate(window).open({ link: "destack://notes--personal--florian/home" }).confirmed;
Window.plan(person.windows, rows); // [{ kind: "show", row }, { kind: "merge", id, into }, ...]
await person.answer({ action: "dialog.answered", id, isAllowed: true }); // grant the view, then show its window
```

## Links

`Link` reads and writes the deep link of a view or an object.

```ts
links.parse("destack://notes--personal--florian/home/drafts");
// { space: "personal--florian", installation: "notes", view: "home", path: "/drafts" }
```

## Menus

`Menu` fills the tray and the application menu section by section with entries, and `menu.offer` offers a declared command on a target meeting its condition.

```ts
await menu.set("autostart", [{ kind: "toggle", id: "toggle", title: "Open at Login", isChecked: true, run }]);
await menu.offer({ command: checkForUpdates }, client, distributionId); // [{ kind: "action", id: "checkForUpdates", title: "Check for Updates…", run }], or []
await menu.choose("autostart.toggle"); // the shell's menu.chosen, by the item's stable id
```

## Serving

`Tray` shows which spaces this computer serves, and whether it syncs or pauses.

```ts
Tray.view({ machine, placements }, open, localization); // placements with their space
// { status: "serving", tooltip: "Destack: Serving 2 spaces", entries: [label, "notes", "Personal"] }
```

## Settings

The `start`, `shortcut` and `autostart` settings set the first window's link, the palette's shortcut and whether the desktop opens at login.

```ts
await native.set(autostart, false); // autostart.set { isEnabled: false }, "Open at Login" unchecked
```

## Banners and the badge

A `Banner` is a desktop message to this client whose notification is unread, recorded received once shown, and the `Badge` shows the inbox's unread count.

```ts
const banners = Banner.select(messages, inbox, client); // the client's messages whose notification is unread
Banner.plan(person.banners, banners, BannerCapabilities.of("darwin")); // [{ action: "banner.show", ... }, { action: "banner.withdraw", id }]
await banner.act(session, "reply", "On my way"); // the activity's act, as the person
```

## Command palette

The palette is a `window` row of kind `palette`: Home's command view floating over the focused window, which closes once it loses focus.

```ts
await desktop.openPalette(); // window.open { kind: "palette", url: "http://home--personal--florian.localhost:7470/command/?focus=…" }
```

## Updates

The update section follows the machine's distribution and offers update's `checkForUpdates` while the release is current and `restartToUpdate` once one is staged.

```ts
await menu.follow("update", distributions, distributions.query.distribution.findFirst().watch(signal), [
    { command: checkForUpdates, after: tellCurrent }, // banner "Destack is up to date" once nothing staged
    { command: restartToUpdate },
]);
```

## Language

The desktop shows its menus, tray and banners in the host's language, as native applications do, and renders declared commands' titles the same way.

```ts
desktop.localization.render(t`Sign Out`); // "Sign Out", or the host language's catalog entry
```

## Failures

The desktop throws `DesktopError` and reports what no event answers to the observability of the signed-in person's home, else to the host's standard error.

```ts
desktop.report("inbox.failed", error); // an exception in the person's home, or stderr before a sign-in
```
