# @destack/view

Build a package's views with Solid, reading and changing its space's objects live.

## Overview

`@destack/view` is Next.js-style views on Solid 2: declared views, live queries over the space's objects, and static sites with islands.

Packages declare views:

- `defineView`: a page of a package, with the object types it opens and presents.

Components read and change objects live:

- `useSpace`, `useAccount` and `useHome`: typed access to the space's, account's or home's objects.
- `useQuery`: a relational query, followed live.
- `useForm` and `useText`: a method's input as a form, and a collaborative text field.
- `useSetting`: a person's setting, followed live.

Pages render and route:

- `@destack/view/router`: Solid Router, with typed search parameters after nuqs.
- `@destack/view/document`: fonts, images, metadata and appearance, as in Next.js.
- `@destack/view/primitives/*`: browser primitives, such as `clipboard` or `resize-observer`.

Declare a notes view that lists and renames notes live:

```tsx
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";
import { useQuery, useSpace } from "@destack/view";
import { defineView } from "@destack/view/declare";

// declare a note type
const note = defineObject({
    name: "note",
    plural: "notes",
    title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
    container: "scope",
    fields: { title: field.string() },
    traits: [shareable({})],
});

/** List the notes, live. */
function Notes() {
    // read the notes in title order, and rename one
    const space = useSpace({ note });
    const notes = useQuery(space.query.note.findMany({ orderBy: { title: "asc" } }));
    const rename = (id: string) => space.mutate.note.update({ id, title: "Groceries" });

    return <p>{notes().length} notes</p>;
}

// declare the view, opening and presenting notes
export const notes = defineView({
    name: "notes",
    title: t`Notes`,
    categories: ["main"],
    objects: [note],
    presents: [{ object: note, priority: "default" }],
    component: async () => ({ default: Notes }),
});
```

## Reference

### defineView

`defineView(definition)` declares a view.

| Option          | What it is                                                  |
| --------------- | ----------------------------------------------------------- |
| `name`, `title` | the view's name, and its title as people read it            |
| `objects`       | the object types it reads and changes                       |
| `presents`      | the object types it presents, and how strongly              |
| `categories`    | the roles clients open it in, such as `"main"`              |
| `audience`      | a signed-in person by default, or anyone                    |
| `packages`      | the packages it calls as its user                           |
| `component`     | loads the module whose default export is the root component |
| `render`        | `{ mode: "client" }` by default, or `{ mode: "static", … }` |

A static view has an `origin`, `pages` with its paths, an optional `document`, and `hydrate`: `"page"` by default, or `"islands"`.

### Live data

`useSpace({ note })` answers `query.note.findMany(…)`, as in `@destack/db`, and `mutate.note.update(…)`, as in `@destack/object`.
`useQuery(query)` follows a query, pending until the replica has it, and writes show at once.

### Primitives

Each primitive is `@destack/view/primitives/<name>`, after Solid Primitives: `active-element`, `audio`, `bounds`, `clipboard`, `connectivity`, `cursor`, `devices`, `event-dispatcher`, `event-listener`, `event-props`, `favicon`, `focus`, `fullscreen`, `geolocation`, `idle`, `input-mask`, `intersection-observer`, `keyboard`, `media`, `mediastream`, `mutation-observer`, `notification`, `orientation`, `page-visibility`, `permission`, `pointer`, `raf`, `resize-observer`, `scheduled`, `scroll`, `selection`, `sensors`, `share`, `storage`, `styles`, `timer`, `upload`, `utils`, `vibrate`, `video`.

## Examples

### Edit an object with a form

`useForm` edits one method call's input, checked by the method's input schema.
`useBlocker` holds navigation while the form has unsaved edits.

```tsx
import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Show, useForm, useSpace } from "@destack/view";
import { useBlocker } from "@destack/view/router";

// declare notes with a short title
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        container: "scope",
        fields: { title: field.string(schema.string().min(1).max(20)) },
        access: { permissions: ["write"] },
    },
    (method) => ({ update: method.update("write") }),
);

/** Rename a note, asking before leaving with unsaved edits. */
export function RenameNote(properties: { readonly id: string; readonly title: string }) {
    // edit the update's input, submitted on save
    const space = useSpace({ note });
    const form = useForm(note, "update", {
        values: () => ({ id: properties.id, title: properties.title }),
        submit: (input) => space.mutate.note.update(input),
    });
    const title = form.field("title");

    // hold navigation while edits are unsaved
    const blocker = useBlocker(() => form.isEdited());

    return (
        <form onSubmit={(event) => (event.preventDefault(), form.submit())}>
            <input
                value={title.value()}
                onInput={(event) => title.set(event.currentTarget.value)}
            />
            <p>{title.problem()}</p>
            <button>Save</button>
            <Show when={blocker.state() === "blocked"}>
                <p>Leave without saving?</p>
                <button type="button" onClick={blocker.proceed}>
                    Leave
                </button>
                <button type="button" onClick={blocker.reset}>
                    Stay
                </button>
            </Show>
        </form>
    );
}
```

### Set a page's metadata

`Metadata` writes the head over the metadata of the pages around it, as in Next.js.

```tsx
import { Metadata } from "@destack/view/document";

/** Title the pricing page, under the site's title template. */
export function PricingPage() {
    return (
        <Metadata title="Pricing" description="Plans for every team.">
            <h1>Pricing</h1>
        </Metadata>
    );
}
```

### Load a third-party script

`Script` loads a script once, however many components ask for it.

```tsx
import { Script } from "@destack/view/document";

/** Load the analytics script after the page hydrates. */
export function Analytics() {
    return <Script src="https://example.com/analytics.js" onReady={() => console.log("ready")} />;
}
```

### Keep state in the address

`useQueryState` keeps a value in the search parameters, as nuqs does.

```tsx
import { parseAsInteger, useQueryState } from "@destack/view/router";

/** Show the page number kept in the address. */
export function Pages() {
    const [page, setPage] = useQueryState("page", parseAsInteger.withDefault(1));

    return <p>page {page()}</p>;
}
```

### Read the person's setting

`useSetting` reads this device's value first.

```tsx
import { appearance } from "@destack/view/setting";
import { useSetting } from "@destack/view";

/** Show the person's appearance setting. */
export function Appearance() {
    const value = useSetting(appearance);

    return <p>{value() ?? "system"}</p>;
}
```

### Prerender a static site

A static view prerenders its pages, and only its islands hydrate.

```tsx
import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";

// prerender two pages, hydrating only islands
export const site = defineView({
    name: "site",
    title: t`Site`,
    component: async () => ({ default: () => <p>Hello</p> }),
    render: {
        mode: "static",
        origin: "https://example.com",
        hydrate: "islands",
        document: async () => ({ default: () => "<!doctype html>" }),
        pages: async () => ({ paths: ["/", "/about/"] }),
    },
});
```

## Comparison

### Next.js

Views, documents, fonts, images and metadata follow Next.js, on Solid 2.

| Next.js                 | `@destack/view`              | Difference                                       |
| ----------------------- | ---------------------------- | ------------------------------------------------ |
| pages and layouts       | `defineView`                 | A view declares the object types it opens.       |
| `fetch` and server data | `useSpace`, `useQuery`       | Queries stay current as the space's rows change. |
| server actions          | `mutate`                     | Writes show at once, and settle on the server.   |
| static export           | `render: { mode: "static" }` | Hydrates only islands, as Astro does.            |
