# @app/notes

Keep notes in notebooks, shared with the people who need them.

## Objects

`notebook` and `note` are the package's object types, and a note in a notebook inherits the grants of the notebook.

```ts
const created = await client.note.create({
    spaceId,
    requestId,
    parentId: notebookId,
    title: "Plan",
});
await client.note.grant({ spaceId, id: created.id, requestId, relation: "editor", subject: bob });
```

### Trash

`note.delete` moves a note to the trash for 30 days, and `note.restore` brings it back for a person who manages it.

```ts
await client.note.delete({ spaceId, id: created.id, requestId });
await client.note.restore({ spaceId, id: created.id, requestId });
```

## Service

`notesService` serves notebooks and notes with their branches.

```ts
import { notesService } from "@app/notes/service";
```

## Views

The `notes` view lists notebooks and notes and opens a note in a live editor.

```ts
import { notes } from "@app/notes/view";
```

## Tables

`notesTables` adds the note tables to another package's database, and `install(notes, …)` installs the package with its `main` database.

```ts
import notes from "@app/notes/package";

export const personal = defineSpace({ installations: { notes: install(notes, { main: "main" }) } });
```
