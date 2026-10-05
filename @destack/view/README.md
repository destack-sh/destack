# @destack/view

Render interfaces with Solid 2.

## Usage

The package root exports the Solid reactive runtime.

```ts
import { createSignal } from "@destack/view";
import { render } from "@destack/view/render";
```

## Views

`defineView` declares a view with the object types it opens and the permissions it requests.

```ts
import { defineView } from "@destack/view/declare";

export const notes = defineView({
    name: "notes",
    objects: [notebook, note, notification, member],
    permissions: {
        space: [notebook.permission("read"), note.permission("read")],
        home: [notification.permission("read")],
        account: [member.permission("read")],
    },
    presents: [{ object: note, priority: "default" }],
    component: () => import("./app.tsx"),
});
```

## Platform services

`useService` calls a platform service that the view declares in `services`.

```tsx
export const issues = defineView({
    name: "issues",
    objects,
    permissions,
    services: [monitorService],
    component,
});

const monitor = useService(monitorService);
const page = createMemo(() =>
    monitor.search({ scope, attributes: { "destack.issue": id }, from, before, limit }),
);
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

## Context

`useView` returns the view's installation, space, person and target object.

```tsx
import { useView } from "@destack/view";

const { space, user, home, locale, target } = useView();
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

## Query updates

`useQuery` re-renders only the rows that changed.

```tsx
const notes = useQuery(() => space.query.note.findMany()); // a failure goes to the nearest error boundary
await space.mutate.note.update({ id, title }).confirmed; // predicted at once, confirmed by the server
await space.mutation(async (mutation) => edit(mutation)).predicted; // one atomic step that undo reverts together
const account = useAccount({ member }); // the account's objects its account permissions grant
const body = useText(note, id, "body"); // one text field, with every keystroke shared live
```

## Permissions and calls

`can` checks a permission of the person on an object, and `call` calls a method by name.

```tsx
const space = useSpace({ note });
if (await space.can("write")) {
    await space.call(note, "publish", { id }).confirmed;
}
```

## Home

`useHome` reads objects from the person's home space.

```tsx
import { useHome } from "@destack/view";

export const inbox = defineView({
    name: "inbox",
    objects: [notification],
    permissions: { home: [notification.permission("read")] },
    component,
});
const home = useHome({ notification });
const unread = useQuery(() => home.query.notification.aggregate(UNREAD));
```

## Forms

`@destack/view/form` binds a `@destack/ui` field to an object field.

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

`StateTransition` shows an object's state and a button per allowed transition.

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

`ReferenceCombobox` picks an object of a type, searching as the person types.

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

`CommandButton` runs an object method and shows a refusal as a toast.

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

`urlOf` returns the address that opens an object in the view presenting it.

```tsx
import { urlOf, useView } from "@destack/view";

const { space } = useView();
<a href={urlOf(note.reference(space, id))}>Open</a>;
<a href={urlOf(note.reference(space, id), { view: "editor" })}>Edit</a>;
```

## Locations

`@destack/view/router` moves a view between locations of its page.

```tsx
import { createRouter, defineRoutes } from "@destack/view/router";

const routes = defineRoutes([{ path: "/", component: () => <h1>Notes</h1> }]);
export const Router = createRouter({ routes });
```

## Language

`useLocale` renders text in the person's language from the package's catalogs.

```tsx
import { useLocale } from "@destack/locale/solid";

const language = document.documentElement.lang; // "de-AT", set by the person or negotiated by their browser
useLocale().render(t`Saved`); // "Gespeichert" with the package's German catalog
```

## Theme

The host writes the theme as custom properties on `<html>`.

```ts
import { defineTheme } from "@destack/theme/declare";

export const theme = defineTheme({ name: "notes", gray: "sand", accent: "orange" });
```

## Launch

`ViewLaunch` parses the launch JSON of the page's `destack-view` script element.

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

`viewExtension` compiles Solid components and emits each view as a `./view/<name>` browser chunk.

```ts
const { entrypoint, permissions, presents } = build.manifest.outputs.browser.views.notes;
```

## Tests

`renderView` from `@destack/view/test` renders a component as a mounted view.

```tsx
import { defineConfiguration, renderView } from "@destack/view/test";

export default defineConfiguration({ test: { include: ["tests/*.test.tsx"] } });
const unmount = renderView(
    Notes,
    element,
    context,
    { [context.space]: client },
    catalogs,
    services,
);
```
