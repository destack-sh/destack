Keep notes in notebooks, shared with the people who need them.

A note is private to its owner until the owner shares it or its notebook.

| Relation | Notebook `read` | Notebook `edit` | Notebook `manage` | Note `read` | Note `edit` | Note `manage` |
|---|---|---|---|---|---|---|
| notebook `owner` | yes | yes | yes | yes | yes | yes |
| notebook `editor` | yes | yes | | yes | yes | |
| notebook `viewer` | yes | | | yes | | |
| note `owner` | | | | yes | yes | yes |
| note `editor` | | | | yes | yes | |
| note `viewer` | | | | yes | | |

A note's `body` is text its editors write together, and a deleted note stays in the trash for 30 days.

```ts
const created = await client.note.create({ spaceId, requestId, parentId: notebookId, title: "Plan" });
await client.note.grant({ spaceId, id: created.id, requestId, relation: "editor", subject: bob });
```

The `notes` view lists notebooks and notes and edits the open note live.

```ts
import { notes } from "@destack/notes/view";
```

A stack installs the package with the database its notes live in, and another package embeds `notesTables` in a database of its own.

```ts
import notes from "@destack/notes/package";

export const personal = defineSpace({ installations: { notes: install(notes, { main: "main" }) } });
```
