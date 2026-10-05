# @destack/view

Render interfaces with Solid 2.

## Usage

The package root exports the Solid reactive runtime, and `@destack/view/render` exports `render`.

```ts
import { createSignal } from "@destack/view";
import { render } from "@destack/view/render";
```

## Views

`defineView` declares a view with the object types it opens, the permissions it requests and the object types it presents, and opening an object picks the view that presents its type.

```ts
import { defineView } from "@destack/view/declare";

export const notes = defineView({
    name: "notes",
    objects: [notebook, note],
    permissions: [note.permission("read")],
    presents: [{ object: note, priority: "default" }],
    component: () => import("./app.tsx"),
});
```

## Commands

`defineCommand` names a call of one method that the shell offers in its command menu, menus, shortcuts, the terminal and agents.

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

## Context

`useView` returns what the host gives the view: its installation, space, account, name, person, home space, locale and the object it opens for.

```tsx
import { useView } from "@destack/view";

const { space, user, home, locale, target } = useView();
```

## Data

`useSpace` opens the objects of the view's space, `useQuery` follows a query, and `mutate`, `mutation` and `undo` change the objects.

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

## Query updates

`useQuery` stays pending until the scope's copy holds the rows and keeps the objects of unchanged rows, so a list re-renders only the rows that changed.

```tsx
const notes = useQuery(() => space.query.note.findMany()); // a failure goes to the nearest error boundary
await space.mutate.note.update({ id, title }).confirmed; // predicted at once, confirmed by the server
await space.mutation(async (mutation) => edit(mutation)).predicted; // one atomic step that undo reverts together
const account = useAccount({ repository }); // the account's objects, read as useSpace reads the space's
const body = useText(note, id, "body"); // one text field, with every keystroke shared live
```

## Permissions and calls

`can` checks a permission of the person on an object, the view's target by default, and `call` calls a mutating method by name.

```tsx
const space = useSpace({ note });
if (await space.can("write")) {
    await space.call(note, "publish", { id }).confirmed;
}
```

## Home

`useHome` reads the object types a view declares in `home` from the person's home space, which the host opens beside the view's space and account.

```tsx
import { useHome } from "@destack/view";

export const inbox = defineView({ name: "inbox", home: [notification], component });
const home = useHome({ notification });
const unread = useQuery(() => home.query.notification.aggregate(UNREAD));
```

## Forms

`@destack/view/form` binds `@destack/ui`'s field to an object field: the field's type picks the control, its schema validates each value in the person's language, and its status follows the optimistic write.

```tsx
import { Field } from "@destack/view/form";

<Field
    for={{
        object: note,
        field: "title",
        value: current().title,
        write: (title) => space.mutate.note.update({ id, title }),
    }}
    label="Title"
/>;
// string → Input, integer and number → number Input, boolean → Switch, time → datetime-local Input,
// enum → Select, state → StateTransition with `id` and `access`; other types need their own control
```

## State transitions

`StateTransition` shows an object's state and a button for each transition its machine allows from it and the person may make.

```tsx
import { StateTransition } from "@destack/view/form";

<StateTransition
    access={space}
    object={note}
    id={id}
    field="status"
    value={current().status}
    labels={{ publish: "Publish" }}
/>;
```

## Reference combobox

`ReferenceCombobox` picks an object of a type through the client's query, searching as the person types.

```tsx
import { ReferenceCombobox } from "@destack/view/form";

<ReferenceCombobox
    aria-label="Notebook"
    search={(name) =>
        space.query.notebook.findMany({ where: { name: { like: `${name}%` } }, limit: 20 })
    }
    label={(notebook) => notebook.name}
    value={notebook()}
    onValueChange={move}
/>;
```

## Command button

`CommandButton` runs an object method, busy until the server confirms it and toasting a refusal.

```tsx
import { CommandButton } from "@destack/view/form";

<CommandButton
    run={() => space.call(note, "archive", { id })}
    errorMessage="The note could not be archived"
>
    Archive
</CommandButton>;
```

## Opening objects

`urlOf` writes the same-origin address at which the host opens an object in the view presenting it most strongly, or in the view asked for, with the object as that view's `target`.

```tsx
import { urlOf, useView } from "@destack/view";

const { space } = useView();
<a href={urlOf(note.reference(space, id))}>Open</a>;
<a href={urlOf(note.reference(space, id), { view: "editor" })}>Edit</a>;
```

## Locations

A view's location is its page's address, which links and `@destack/view/router` move through.

```tsx
import { createRouter, defineRoutes } from "@destack/view/router";

const routes = defineRoutes([{ path: "/", component: () => <h1>Notes</h1> }]);
export const Router = createRouter({ routes });
```

## Language

The host writes the person's language into `<html lang>` and launches the view with the catalogs of its package and dependency releases along that language's fallback chain, which `useLocale` renders in.

```tsx
import { useLocale } from "@destack/locale/solid";

const language = document.documentElement.lang; // "de-AT", set by the person or negotiated by their browser
useLocale().render(t`Saved`); // "Gespeichert" with the package's German catalog
```

## Theme

The host writes the view package's declared theme, or Destack's default, for the person's display settings as custom properties on `<html>`, and pushes each change on `/.destack/display`.

```ts
import { defineTheme } from "@destack/theme/declare";

export const theme = defineTheme({ name: "notes", gray: "sand", accent: "orange" });
```

## Launch

`ViewLaunch` parses the JSON in the page's `destack-view` script element, and mounting a view starts the page's telemetry with its `release` and `manifest`.

```ts
import { ViewLaunch } from "@destack/view/declare";

const launch = ViewLaunch.parse(JSON.parse(element.textContent));
// { installation, space, account, view, user, home?, target?, locale?, release, manifest?, endpoint, catalogs }
```

## Document

`@destack/view/document` exports Solid Meta 1, with `Head`, `Title` and `Meta`.

```tsx
import { Head, Meta, Title } from "@destack/view/document";

export function Metadata() {
    return (
        <Head>
            <Title>Notes</Title>
            <Meta name="description" content="Your notes." />
        </Head>
    );
}
```

## Builds

`viewExtension` compiles Solid components wherever modules of packages depending on `@destack/view` compile, as the `SolidApplication` a web output names, and emits each view as a separate `./view/<name>` browser chunk.

```ts
const { entrypoint, permissions, presents } = build.manifest.outputs.browser.views.notes;
```

## Tests

`defineConfiguration` from `@destack/view/test` compiles components with the transforms of the package's dependency closure as builds do, and `renderView` renders a component as a mounted view over its scopes' clients.

```tsx
import { defineConfiguration, renderView } from "@destack/view/test";

export default defineConfiguration({ test: { include: ["tests/*.test.tsx"] } });
const unmount = renderView(Notes, element, context, { [context.space]: client }, catalogs);
```
