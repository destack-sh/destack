Keep notes in notebooks, shared with the people who need them.

## Notes

Notes and notebooks use the default shareable roles, and a note's `body` is text its editors write together.

```ts
const created = await client.note.create({
    spaceId,
    requestId,
    parentId: notebookId,
    title: "Plan",
});
await client.note.grant({ spaceId, id: created.id, requestId, relation: "editor", subject: bob });
```

A note may stand outside any notebook, and a deleted note stays in the trash for 30 days.

## View

The `notes` view lists notebooks and notes and edits the open note live.

```ts
import { notes } from "@destack/notes/view";
```

## Installation

A stack installs the package with the database its notes live in, and another package embeds `notesTables` in a database of its own.

```ts
import notes from "@destack/notes/package";

export const personal = defineSpace({ installations: { notes: install(notes, { main: "main" }) } });
```
