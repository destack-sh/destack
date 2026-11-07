# @destack/home

Home is the first view the desktop opens, installed in the home space of every person's personal account.

## Home

`home` lists the account's spaces by name, each with the views its installations mount, opening one through the page's open path.

```ts
export const home = defineView({
    name: "home",
    categories: ["home"],
    objects: [space],
    component: () => import("./home.tsx"),
});

<Spaces /> // "Personal · Connected · home--florian", then its views: "Notes", "Tasks"
```

## Approvals

Each space shows the plans its installations wait on, and the person approves one by the digest they reviewed, an access grant among them.

```ts
const waiting = await readWaitingPlans(spaceId, origin, environment); // installations whose progress is "approval"
waiting[0]; // { name: "notes", plan: { steps: [{ action: "grant", risk: "privileged", detail: "request read on space" }] }, digest }
await approveWaitingPlan(waiting[0], origin, environment); // rollout.approve at the space's stack
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

`command` is the command palette, Home's view the desktop opens over a window: `PaletteListing` reads each space's `SpaceCatalog` from one `ObjectStoreDescription` of its store, and the device's windows and recent objects at the device's home.

```ts
const listing = await PaletteListing.read(
    origin,
    { user, home, device },
    { keybindings, sources },
    Clock.system,
);
const catalog = listing.catalog(entry); // the space's installations, commands, views and types
catalog.command(entry); // the Command, bound to the type it calls
await catalog.call(entry, input, id); // through the installation, at /.destack/home/<space>
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
