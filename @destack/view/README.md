# @destack/view

Build a package's views with Solid, reading and changing its space's objects live.

## Overview

`@destack/view` is Next.js-style views on Solid 2: declared views, live queries over the space's objects, and static sites with islands.

Packages declare views:

- `defineView`: a page of a package, with the object types it opens and presents.
- `handle` and `handleOnPage`: run an action `@destack/object` declares, in an element or on the page.
- `provide` and `provideOnPage`: give the place of an element, or of the page, to actions.

Components read and change objects live:

- `useQuery`: a relational query of an object type, followed live.
- `useMutation`: a mutating method of an object type, predicted at once.
- `useForm` and `useText`: values as a form edits them, and a collaborative text field.
- `useSetting`: a person's setting, followed live.
- `useAction`: what an action does at a place, as buttons and menus draw it.

Pages render and route:

- `@destack/view/router`: Solid Router, with typed search parameters after nuqs.
- `@destack/view/document`: fonts, images, metadata and appearance, as in Next.js.
- `@destack/view/primitives/*`: browser primitives, such as `clipboard` or `resize-observer`.

Scenarios play a package's examples:

- `viewInteraction`: the steps and observations of a UI scenario.
- `ViewDriver`: plays scenarios in a test DOM.
- `BrowserDriver`: plays scenarios in Chromium, each in its package's frame.

Declare a notes view that lists and renames notes live:

```tsx
import { argument, plural, t } from "@destack/locale";
import { defineObject, field, shareable } from "@destack/object";
import { useMutation, useQuery } from "@destack/view";
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
    const notes = useQuery(note, (query) => query.findMany({ orderBy: { title: "asc" } }));
    const update = useMutation(note, "update");
    const rename = (id: string) => update({ id, title: "Groceries" });

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

`useQuery(note, (query) => query.findMany(…))` follows a query of notes, as in `@destack/db`, on the person's device.
`useMutation(note, "update")` calls a method of notes, as in `@destack/object`, its prediction showing at once.
Each reads and writes the scope the type lives in for the view, or `{ scope }`: `"home"`, `"account"`, or a space's identifier.
`useFiles(note)` sends files of notes and writes their addresses, as `ObjectFileClient` in `@destack/object`.

### Actions

`useActions()` returns the page's actions, which menus, buttons, keys and the palette run.
A key of a longer shortcut such as `"g m"` waits for the next, and `sequence.progress()` reads what can follow.

| Verb                             | What it does                                                         |
| -------------------------------- | -------------------------------------------------------------------- |
| `run(action, element, given?)`   | runs it at an element, asking what neither places nor `given` answer |
| `start(action, element, given?)` | runs it as a person does, telling them where it fails                |
| `at(element)`                    | lists every action there, with the methods of the objects            |
| `on(element)`                    | lists the actions on the nearest row, card or page                   |
| `subjectAt(element)`             | finds that row, card or page                                         |
| `has(reference, element)`        | reports whether it runs there                                        |
| `keybindings(action, isCommand)` | lists the person's keys for it, else its own                         |

`handleAsking` asks the person for what is missing, and `handleFailing` tells what failed, as `ActionDialog` and `Toaster` of `@destack/ui` do.

### Handlers

`handle(action, handler)` handles an action in the element it is the `ref` of, and `handleOnPage` on the page.
The innermost enabled handler wins, as a DOM event bubbles.

| Field       | What it says                                  | Without it                         |
| ----------- | --------------------------------------------- | ---------------------------------- |
| `run`       | what the action does                          | the action's `call`                |
| `isEnabled` | whether it runs, else its keys go to the page | it does                            |
| `choices`   | the values a person picks an input from       | the field's, or the type's objects |
| `chosen`    | the values an input has                       | the values the call's rows share   |
| `subject`   | the title of what it runs on                  | the object's title                 |
| `isOn`      | whether what it turns on is on                | it turns on nothing                |

### Places

`provide(place)` gives a `Place` to the actions in the element it is the `ref` of, and `provideOnPage` to the page's.
An action takes the nearest place around the person that gives what it needs.

### useAction

`useAction(() => ({ action, element }))` tells what an action does at an element, and follows its changes.

| Field                     | What it reads                                       |
| ------------------------- | --------------------------------------------------- |
| `isEnabled()`, `isOn()`   | whether it runs, and whether what it turns on is on |
| `input()`                 | the one input left that a person picks from a list  |
| `choices()`               | what they pick it from                              |
| `chosen()`, `isChosen(…)` | the values it has, as they change                   |
| `start(value?)`           | runs it, with the picked value                      |

### Standard actions

Every page handles the standard actions, which the `keybindings` setting rebinds by `<package>/<name>` as VS Code's `keybindings.json` does.

| Action         | Keys    | What it does                                                     |
| -------------- | ------- | ---------------------------------------------------------------- |
| `undo`, `redo` | ⌘Z, ⇧⌘Z | undo or redo the person's newest change across the page's scopes |
| `palette`      | ⌘K      | open the command palette, from fields too                        |

### Scenarios

A UI scenario takes these steps.

| Step                                      | Does                                |
| ----------------------------------------- | ----------------------------------- |
| `focus`                                   | moves the focus to an element       |
| `click`                                   | clicks an element                   |
| `pointerMove`, `pointerDown`, `pointerUp` | move, press and release the pointer |
| `wheel`                                   | turn the wheel where the pointer is |
| `press`                                   | presses a key, such as `Control+r`  |
| `fill`                                    | replaces a field's value            |
| `pause`                                   | lets time pass                      |

A `*.scenario.tsx` module plays in a test DOM.
A `*.browser.scenario.tsx` module plays in Chromium.

### Primitives

Each primitive is `@destack/view/primitives/<name>`, after Solid Primitives, such as `clipboard`, `media` or `resize-observer`.

## Examples

### Bold the focused editor's selection

`handle` runs `bold` for the editor that has focus, while it has a selection.

```tsx
import { t } from "@destack/locale";
import { handle } from "@destack/view";
import { defineAction } from "@destack/object";

// declare bolding the selected text
export const bold = defineAction({
    name: "bold",
    title: t`Bold`,
    category: t`Format`,
    keybinding: "mod+b",
});

/** Bold this editor's selection on the person's keys or from the palette, while it has focus. */
export function Editor(properties: {
    readonly hasSelection: () => boolean;
    readonly bold: () => void;
}) {
    return (
        <textarea
            ref={handle(bold, {
                run: () => properties.bold(),
                isEnabled: properties.hasSelection,
            })}
        />
    );
}
```

### Draw an action's choices

`useAction` lists what an action takes, and marks what it has.

```tsx
import type { Action } from "@destack/object";
import { For, useAction } from "@destack/view";

/** Render the choices of an action on the page as buttons. */
export function Choices(properties: { readonly action: Action }) {
    const own = useAction(() => ({ action: properties.action, element: null }));

    return (
        <For each={own.choices()}>
            {(choice) => (
                <button
                    aria-pressed={own.isChosen(choice) ? "true" : "false"}
                    onClick={() => own.start(choice.value)}
                >
                    {choice.title}
                </button>
            )}
        </For>
    );
}
```

### Edit values with a form

`useForm` edits some values, checks them by a schema and gives them to `submit`, which calls a method or does anything else.
`useBlocker` holds navigation while the form has unsaved edits.

```tsx
import { schema } from "@destack/schema";
import { Show, useForm } from "@destack/view";
import { useBlocker } from "@destack/view/router";

/** A note's title. */
const Title = schema.object({ title: schema.string().min(1).max(20) });

/** Rename a note, asking before leaving with unsaved edits. */
export function RenameNote(properties: {
    readonly title: string;
    readonly rename: (title: string) => Promise<void>;
}) {
    // edit the title, submitted on save
    const form = useForm({
        values: () => ({ title: properties.title }),
        schema: Title,
        submit: ({ title }) => properties.rename(title),
    });
    const title = form.field("title");

    // hold navigation while edits are unsaved
    const blocker = useBlocker(() => form.isEdited());

    return (
        <form onSubmit={(event) => (event.preventDefault(), form.submit())}>
            <input
                value={title.value()}
                onInput={(event) => title.set(event.currentTarget.value)}
                onBlur={() => title.settle()}
            />
            <p>{title.problem()}</p>
            <button>Save</button>

            {/* what someone else named it meanwhile */}
            <Show when={title.conflict()}>
                {(theirs) => (
                    <button type="button" onClick={() => title.take()}>
                        Use “{theirs().value}”
                    </button>
                )}
            </Show>
            <Show when={blocker.state() === "blocked"}>
                <button type="button" onClick={blocker.proceed}>
                    Leave without saving
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

### Show a person's theme

`theme` is the theme a person sees in every app, Destack's when null, and a page's visitors see its package's theme instead.

```ts
import { theme } from "@destack/view/setting";
import { titanTheme } from "@destack/theme/declare";

// change Titan's page
export const chosen = theme.definition.schema.parse({
    ...titanTheme.definition,
    ground: {
        light: { background: "#fff8e7", foreground: "#3d5157", muted: "#f1e9d2" },
        dark: { background: "#00242e", foreground: "#eee8d5", muted: "#06323d" },
    },
});
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

### Hold with a finger in a browser

Set the `pointer` global to `touch` to play a scenario with a finger.

```tsx
import { defineExample, defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";

// a note's title as a button, and an example of it
const Note = (properties: { readonly title: string }) => <button>{properties.title}</button>;
export const noteGroceries = defineExample({
    of: Note,
    name: "groceries",
    description: "a note titled Groceries",
    render: () => <Note title="Groceries" />,
});

// hold the note under a finger for half a second, then let go
export const noteHold = defineScenario({
    of: Note,
    interaction: viewInteraction,
    name: "hold",
    description: "hold a note under a finger for half a second and let go on it",
    given: { examples: [noteGroceries], globals: { pointer: "touch" } },
    parameters: {},
    when: [
        { step: { action: "pointerMove", target: { role: "button", name: "Groceries" } } },
        { step: { action: "pointerDown" } },
        { step: { action: "pause", milliseconds: 500 } },
        {
            step: { action: "pointerUp" },
            then: [{ observe: { kind: "focused" }, is: "Groceries" }],
        },
    ],
});
```

### Frame an example

An origin serves a development build's frame at `OriginPath.frame`, rendering the example or scenario a `module` and `name` select.

```ts
import { OriginPath } from "@destack/package";

export function frameOf(origin: string): string {
    const query = new URLSearchParams({ module: "src/note/note.example.tsx", name: "groceries" });

    return `${origin}${OriginPath.frame}?${query}`; // Storybook's iframe.html?id=
}
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
