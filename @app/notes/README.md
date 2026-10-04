# @app/notes

Keep notes in notebooks, shared with the people who need them.

## Notes

`note.create` creates a note in the notebook `parentId` names, or outside any notebook without it, and `note.grant` gives a person a role on the note.

```ts
const created = await client.note.create({
    spaceId,
    requestId,
    parentId: notebookId,
    title: "Plan",
});
await client.note.grant({ spaceId, id: created.id, requestId, relation: "editor", subject: bob });
```

## Trash

`note.delete` moves a note to the trash for 30 days, and `note.restore` brings it back for a person who manages it.

```ts
await client.note.delete({ spaceId, id: created.id, requestId });
await client.note.restore({ spaceId, id: created.id, requestId });
```

## View

The `notes` view lists notebooks and notes and opens a note in a live editor.

```ts
import { notes } from "@app/notes/view";
```

## Installation

`install(notes, …)` installs the package with its `main` database, and `notesTables` adds the note tables to another package's database.

```ts
import notes from "@app/notes/package";

export const personal = defineSpace({ installations: { notes: install(notes, { main: "main" }) } });
```
