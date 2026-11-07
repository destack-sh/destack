# @destack/home

Home is the first view the desktop opens, installed in the home space of every person's personal account.

## Home

`home` lists the account's spaces by name, each with the views its installations mount, opening one through the page's open path.

```ts
export const home = defineView({
    name: "home",
    categories: ["home"],
    objects: [space],
    component: () => import("./app.tsx"),
});

<Spaces /> // "Personal · Connected · home--florian", then its views: "Notes", "Tasks"
```

## Settings

`settings` shows and changes the person's settings.

```ts
export const settings = defineView({ name: "settings", objects: [setting], component });
```

## Start

`start` is the link a new window opens, the person's own or this computer's, which the desktop reads from Home.

```ts
import { start } from "@destack/home/setting";

start.default; // null: Home's start view
// destack setting start set destack://home--florian/home --scope device
```

## Clients

Home binds the shell's credential to this browser's client once it shows, as a person's page registers its browser at its origin.

```ts
await new BrowserClient({ path: CLIENT_PATH, origin, environment }).bind(
    BrowserClient.describe(navigator),
);
```

## Commands

`command` is the command palette, Home's view the desktop opens over a window: `PaletteListing` reads the person's home through its scope's home path, and their device's windows and recent objects at the device's home.

```ts
const listing = await PaletteListing.read(
    origin,
    { user, home, device },
    { keybindings, sources },
    Clock.system,
);
const shape = listing.catalog(command).shape(command); // from the type's describeMethods()
await listing.catalog(command).call(command, { id }); // through the installation, at /.destack/home/<space>
```

## Palette

`CommandPalette` searches the person's windows, commands, views, spaces and recent objects, picks a command's object and fills its input with `SchemaForm`.

```tsx
// offers the command, run once confirmed
<CommandPalette
    listing={listing}
    focus={focus}
    command={{ packageId, name: "complete" }}
    onOpen={open}
    onWindow={focus}
    onClose={close}
/>
```
