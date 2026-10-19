# @destack/view

`@destack/view` renders a package's views in the shell with Solid, and gives them their space's objects, the person's settings and the shell's commands.

```tsx
const space = useSpace({ notebook, note });
const notes = useQuery(() => space.query.note.findMany({ where: { parentId: notebook() } }));
await space.mutate.note.update({ id, title: "Groceries" });
```

## Prior art

`@destack/view` is Solid 2, `/router` is Solid Router 2 with nuqs' `useQueryState` and React Router's `useBlocker`, `/document` follows Next.js's `Metadata`, `next/font` and next-themes, `/media` follows `next/image`, and `/test` follows Testing Library's `render`.

```tsx
const [page, setPage] = useQueryState("page", parseAsInteger.withDefault(1));
const blocker = useBlocker(() => form.isEdited());
const sans = defineFont({ src: plexSans, fallback: ["sans-serif"] });
<Font font={sans} />;
const { resolvedAppearance } = createAppearance();
<Image src={cover} alt="" placeholder="blur" />;
render(() => <Notes />, { container, hydrate: true });
```

## Views

`defineView` declares a view with the object types it opens and the permissions it requests.

```ts
import { defineView } from "@destack/view/declare";

export const notes = defineView({
    name: "notes",
    objects: [notebook, note, notification, member], // each opened where it lives
    presents: [{ object: note, priority: "default" }],
    component: () => import("./app.tsx"),
});
```

## Packages

`usePackage` calls a package that the view declares in `packages`.

```tsx
export const traces = defineView({
    name: "traces",
    objects,
    permissions,
    packages: [observabilityPackage],
    component,
});

const observability = usePackage(observabilityPackage);
const spans = createMemo(() => observability.span.list({ scope, where: `trace = "${traceId}"` }));
```

## Page origin

`useOrigin` calls a path below the page's origin with its session cookie, and the origin calls each path as the person under their grant to the installation.

```ts
import { useOrigin } from "@destack/view";
import { PACKAGE_PATH, PackageMount } from "@destack/runtime";
import { HOME_PATH, PLATFORM_PATH } from "@destack/view/declare";

const origin = useOrigin();
createClient(notesPackage, origin(PACKAGE_PATH)); // the installation's own package
createClient(stackPackage, origin(`${PLATFORM_PATH}${PackageMount.path(stackPackage.id)}`)); // a service its views declare
createClient(settings, origin(`${HOME_PATH}/${userId}`)); // where a change of the person's own rows goes
```

## Launches

A host launches a view at its installation's origin with the person's token, which the origin trades for its session cookie; `ViewAddress` and `ObjectAddress` are what a launch opens.

```ts
import { ViewAddress, type Address } from "@destack/view/declare";

const board: Address = { space: "home--acme", installation: "tasks", view: "board", path: "/" };
const note: Address = { space: "home--acme", object: note.reference(spaceId, id) }; // the view presenting it
ViewAddress.url(board, "https://tasks--home--acme.destack.space", token); // …/board?token=…
```

## Mounting

The origin's document carries the view's `ViewContext`, and the build's module mounts the view with its package's theme and commands.

```ts
ViewContext.document(context, view.entrypoint); // <script id="destack-view">{"installation":…}</script>
await mount(view, { commands: [{ packageId, name: "archive", keybinding: "mod+e" }], theme }); // generated
// catalogs load by package from the build: /locale/de.json, /locale/<package>/de.json
```

## Commands

`defineCommand` declares a method call that the shell offers in menus, shortcuts and the terminal.

```ts
import { defineCommand } from "@destack/view/declare";

export const archive = defineCommand({
    name: "archive-note",
    title: "Archive note",
    object: note,
    method: "archive",
    keybinding: "mod+shift+a",
});
```

## Palette ranking

`Palette.rank` orders entries by the typed text and the focused object.

```ts
import { Palette } from "@destack/view/palette";

const ranked = Palette.rank(entries, "arch", focus);
const binding = Palette.keybinding(command, "mod+e", overrides); // "mod+e" or null
const isOffered = Palette.isOffered(entry, { spaces: false }); // false for a space
```

## Shortcuts

`Accelerator` reads keybindings such as `mod+k`, `mod` being Command on Apple platforms and Control elsewhere.

```ts
import { Accelerator, isCommandPlatform } from "@destack/view/palette";
import { createShortcut } from "@destack/view/primitives/keyboard";

Accelerator.format("mod+shift+k", isCommandPlatform()); // "⌘⇧K" or "Ctrl+Shift+K"
createShortcut(Accelerator.keys("mod+k", isCommandPlatform()), openPalette);
```

## Context

`useView` returns the view's installation, space, person and target object.

```tsx
import { useView } from "@destack/view";

const { space, user, home, locale, target } = useView();
```

## Controllable signals

`createControllableSignal` is Kobalte's: it follows an owner's value while the owner controls it, else its own.

```ts
import { createControllableSignal } from "@destack/view";

const [open, setOpen] = createControllableSignal({
    isControlled: () => properties.open !== undefined,
    value: () => properties.open ?? false,
    defaultValue: false,
    onChange: properties.onOpenChange,
});
```

## Motion

`createReducedMotion` follows whether the theme keeps an element's motion still, from the person's motion setting or the device's preference, where Framer Motion's `useReducedMotion` reads only the device.

```tsx
const isStill = createReducedMotion(canvas);
createTimer(turn, () => (isStill() ? false : 6000), setInterval);
isMotionReduced(element); // one read, for code outside tracking
```

## Data

`useSpace` opens the objects of the view's space, and `useQuery` follows a query.

```tsx
import { For, useQuery, useSpace } from "@destack/view";

const space = useSpace({ notebook, note });
const notes = useQuery(() =>
    space.query.note.findMany({ where: { parentId: notebook() }, orderBy: { title: "asc" } }),
);
await space.mutate.note.update({ id, title: "Groceries" });
await space.mutation(async (mutation) => {
    await mutation.call(note).create({ title: "Milk" });
    await mutation.call(note).create({ title: "Bread" });
}).predicted;
await space.undo().confirmed;
```

## Accounts and text

`useAccount` opens the objects of the view's account, and `useText` follows one text field with every keystroke shared live.

```tsx
const account = useAccount({ member });
const body = useText(note, id, "body");
```

## Permissions and calls

`can` checks a permission of the person on an object, and `call` calls a method by name.

```tsx
const space = useSpace({ note });
if (await space.can("write")) {
    await space.call(note, "publish", { id }).confirmed;
}
```

## Forms

`useForm` edits the input of an object's method call, checked against the method's input schema.

```tsx
const form = useForm(note, "update", {
    values: () => ({ id: note().id, title: note().title }),
    submit: (input) => space.mutate.note.update(input),
    mode: "submit", // or "change"
});
<Field invalid={form.field("title").problem() !== undefined}>
    <Input
        value={form.field("title").value()}
        onInput={(event) => form.field("title").set(event.currentTarget.value)}
    />
    <FieldError>{form.field("title").problem()}</FieldError>
</Field>;
<Button loading={form.status() === "pending"} onClick={() => form.submit()}>
    Save
</Button>;
```

## Schema forms

`useSchemaForm` edits an input any schema checks, such as one read from JSON Schema, and reports refusals by JSON Pointer.

```ts
const form = useSchemaForm(fromJsonSchema(input), { values: () => ({ place: {} }), submit });
form.field("place").set({ city: "" });
form.problems(); // Map { "/place/city" => "Enter a value" }
form.field("place").problem(); // "Enter a value"
```

## Home

`useHome` reads objects from the person's home space.

```tsx
import { useHome } from "@destack/view";

export const inbox = defineView({
    name: "inbox",
    objects: [notification],
    component,
});
const home = useHome({ notification });
const unread = useQuery(() => home.query.notification.aggregate(UNREAD));
```

## Opening objects

`urlOf` returns the address that opens an object in the view presenting it.

```tsx
import { urlOf, useView } from "@destack/view";

const { space } = useView();
<a href={urlOf(note.reference(space, id))}>Open</a>;
<a href={urlOf(note.reference(space, id), { view: "editor" })}>Edit</a>;
```

## Language

`useLocale` reads the person's `Localization`, which `renderView` provides for views and `LocaleContext` for websites.

```tsx
import { plural, t } from "@destack/locale";
import { useLocale } from "@destack/view";

export function Archived(properties: { count: number; notebook: string }) {
    const locale = useLocale();
    const notes = plural(properties.count, { one: "# note", other: "# notes" });

    return <p>{locale.render(t`Archived ${notes} in ${properties.notebook}`)}</p>;
}

<LocaleContext value={Localization.of(Locale.parse("de-AT"), catalogs)}>{page}</LocaleContext>;
```

## Display

`resolveDisplay` resolves a person's `Display` from the `appearance`, `textSize`, `density`, `contrast`, `motion` and `accent` settings, their device's own values first, which a page presents with its theme, in the person's language, beside their keybindings.

```ts
import { resolveDisplay } from "@destack/view/setting";

const display = resolveDisplay(selection, values, chain, device); // device: the device's own values
const style = theme.variables(display);
```

## Styles

`@destack/view/styles` loads a document's base styles: Preflight, the theme's element defaults and the StyleX rules; import it from the app module that server and browser both load, as SolidStart's `app.tsx`, so the browser bundle includes it.

```tsx
import "@destack/view/styles";
```

## Examples

`renderExample` renders an example into an element in a locale, direction, width and theme.

```ts
import { renderExample } from "@destack/view/example";

const unmount = renderExample(element, {
    example: buttonGhost,
    environment: { locale: "ar-EG", direction: "rtl", width: 320, theme: { appearance: "dark" } },
    catalogs,
});
```

## Tests

`renderView` from `@destack/view/test` renders a component as a mounted view.

```tsx
import { defineConfiguration, renderView } from "@destack/view/test";

export default defineConfiguration({ test: { include: ["tests/*.test.tsx"] } });
const unmount = renderView(
    Notes,
    element,
    { context, clients: new Map([[context.space, client]]), packages: new Map(), origin },
    Localization.of("de", catalogs),
);
```

## Scenario interaction

`viewInteraction` from `@destack/view/scenario` is the UI interaction scenarios speak, after Playwright's actions, locators and assertions.

```ts
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";

export const dropdownMenuOpenOntoFirstItem = defineScenario({
    of: DropdownMenu,
    interaction: viewInteraction,
    name: "open-onto-first-item",
    description: "open the note menu onto its first item",
    given: { examples: [dropdownMenuNoteMenu] },
    when: [{ action: "click", target: { role: "button", name: "Note" } }],
    then: { observe: { focused: { kind: "focused" } }, end: { focused: "Rename" } },
});
```

## Scenario tests

`ViewDriver` plays the UI interaction in the test DOM.

```ts
import { ViewDriver } from "@destack/view/test";

const driver = ViewDriver.start({ examples: [dropdownMenuNoteMenu] });
driver.act({ action: "click", target: { role: "button", name: "Note" } });
driver.observe({ kind: "focused" }); // "Rename"
```
