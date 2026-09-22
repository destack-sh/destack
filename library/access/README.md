Declare permissions, evaluate authoritative records, and restrict database queries.

## Usage

```ts
import { defineObject, relation, union } from "@destack/access";

export const note = defineObject({
    packageId: import.meta.destack.package.id,
    name: "note",
    attributes: {},
    relations: {
        owner: { kind: "subject", subjects: ["user"] },
        editor: { kind: "grant", subjects: ["user", "group"], permission: "share" },
    },
    permissions: {
        share: relation("owner"),
        edit: union(relation("owner"), relation("editor")),
    },
});
```

## Database

```ts
import { AccessModel } from "@destack/access";
import { AccessQuery, GrantStore } from "@destack/access/database";
import { accessSchema } from "@destack/access/stack";
import { defineDatabaseSchema } from "@destack/db";

export const notesSchema = defineDatabaseSchema({
    name: "notes",
    tables: { notes },
    dependencies: [accessSchema],
    migrations: new URL("./migration/", import.meta.url),
});

const model = new AccessModel([note]);
const query = new AccessQuery(model, [
    {
        type: note,
        table: notes,
        id: "id",
        scope: "spaceId",
        attributes: {},
        subjects: { owner: { column: "userId", kind: "user", authority: "global" } },
        objects: {},
    },
]);

const rows = await database
    .select()
    .from(notes)
    .where(query.where(note.permission("edit"), spaceId, context))
    .limit(20);

const sharing = new GrantStore(database, query, recordGrantChange);
await sharing.grant(
    {
        object: note.ref(spaceId, noteId),
        relation: "editor",
        subject: { kind: "user", authority: "global", id: recipientId },
    },
    context,
);

// recordGrantChange persists the audit event in the supplied transaction
```

## Memory

```ts
import { Access, AccessSnapshot } from "@destack/access";

const snapshot = new AccessSnapshot(revision, objects, grants, tokens);
const access = new Access(model, snapshot);
access.require(note.permission("edit"), note.ref(spaceId, noteId), context);

// a resident application can supply AccessView directly
// the host verifies identities, memberships, attributes, time and delegation records
```

## Inspection

```ts
import { AccessDeclaration, AccessPolicyDescription } from "@destack/access/inspect";

const declarations = model.describe();
const policies = model.policies;
const decision = access.explain(note.permission("edit"), note.ref(spaceId, noteId), context);
```
