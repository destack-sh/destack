# @destack/view

Build a package's views with Solid, reading and changing its space's objects live.

## Overview

A package declares its views with the object types they open, and each view renders at its installation's origin with its space's objects kept in sync.

- **View**: a page of a package with the object types it opens and the objects it presents; `defineView`.
- **Space access**: a view's typed client of its space's objects; `useSpace`, then `query` and `mutate`, with `useAccount` and `useHome` for the account and the person's home.
- **Query**: a relational query followed live; `useQuery` keeps a component current.
- **Setting**: a person's setting followed live; `useSetting`.
- **Router**: Solid Router with typed search parameters, after nuqs' `useQueryState`.
- **Static view**: pages prerendered at build with islands that hydrate, after Astro.

```tsx
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";
import { useQuery, useSpace } from "@destack/view";
import { defineView } from "@destack/view/declare";

const note = defineObject({
    name: "note",
    plural: "notes",
    title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
    container: "scope",
    fields: { title: field.string() },
    traits: [shareable({})],
});

function Notes() {
    const space = useSpace({ note });
    const notes = useQuery(space.query.note.findMany({ orderBy: { title: "asc" } }));
    const rename = (id: string) => space.mutate.note.update({ id, title: "Groceries" });

    return <p>{notes().length} notes</p>;
}

export const notes = defineView({
    name: "notes",
    title: t`Notes`,
    categories: ["main"],
    objects: [note],
    presents: [{ object: note, priority: "default" }],
    component: async () => ({ default: Notes }),
});
```

## Examples

### Keep state in the address

`useQueryState` keeps a value in the page's search parameters, as nuqs does.

```tsx
import { parseAsInteger, useQueryState } from "@destack/view/router";

function Pages() {
    const [page, setPage] = useQueryState("page", parseAsInteger.withDefault(1));

    return <p>page {page()}</p>;
}
```

### Read the person's setting

`useSetting` follows a person's setting, this device's value first.

```tsx
import { appearance } from "@destack/view/setting";
import { useSetting } from "@destack/view";

function Appearance() {
    const value = useSetting(appearance);

    return <p>{value() ?? "system"}</p>;
}
```

### Prerender a static site

A static view prerenders its pages at build, and only the components in its islands hydrate.

```tsx
import { t } from "@destack/locale";
import { defineView } from "@destack/view/declare";

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

Views, documents, fonts, images and metadata follow Next.js, on Solid 2; four things differ.

- A view declares the object types it opens, and its queries stay current as the space's rows change.
- A view's page lives at its installation's origin, which signs the person in at the universe.
- Writes show at once and settle when the server applies them.
- Static views prerender their pages and hydrate only their islands, as Astro does.
