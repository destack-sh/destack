# @destack/object

Declare object types with fields, traits, permissions and methods, then serve and sync them.

## Objects

`defineObject` declares an object type and derives its table, policy and procedures.

```ts
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    fields: {
        title: field.string(schema.string().min(1)),
        owner: field.reference(principal.user).caller(),
        status: field.state({ initial: "draft", transitions: { publish: { from: ["draft"], to: "published", permission: "write" } } }),
    },
    nested: { in: notebook, receive: "write", move: "write" },
    recoverable: { within: { days: 30 }, by: "write" },
    permissions: { read: union(relation("owner"), through("parent", "read")), write: relation("owner") },
    methods: { get: method.get("read"), list: method.list("read"), create: method.create("write"), update: method.update("write") },
});
```

## Traits

Each trait adds columns, permissions and methods to the type.

| Trait | Options | Adds |
|---|---|---|
| `nested` | `{ in, receive, optional?, delete?, move? }` | The parent reference, and `move` |
| `recoverable` | `{ within, by, purge?, keep? }` | The trash, with `delete`, `restore` and `purge` |
| `expiring` | `[{ after, from, where? }]` | Removal by the system once a rule's window passes |
| `addressed` | `{ recipient }` | A copy in the recipient's home while the recipient may read the object |
| `versioned` | `true` | The version number within the parent |
| `controlled` | `true` | The desired generation, a controller's conditions and the deletion request |
| `declarable` | `{ schema }` | The stack declaration managing the record |
| `detachable` | `{ by }` | `detach` |
| `shareable` | `{ by }` | Relationships, proposals and `explain` |
| `suspendable` | `{ by }` | `suspend` and `resume` on a scope type |
| `tracked` | `{ by, activity, session? }` | Changes grouped into activities, with `history` and `revert` |
| `audited` | `{ reads: true }` | An audit event for each read |

## Methods

`method` declares a method and the permission it requires.

| Builder | Method |
|---|---|
| `method.get`, `method.list` | Read one object, or a page of them |
| `method.create`, `method.update`, `method.delete` | Change one object |
| `method.updateMany` | Update every object matching some fields that the caller may change |
| `method({ permission })` | A custom method, with `isSystem: true` for one only the system executes |

## Releases

A renamed field keeps its column and its earlier callers' inputs, and `convert` computes fields from stored rows and earlier callers' inputs by the release introducing them.

```ts
const note = defineObject({
    name: "note",
    moved: { fields: { body: "text" } },                                            // calls of earlier releases may still send `text`
    convert: { "2026.10.0": { pinned: Expression.coalesce(Expression.column("pinned"), Expression.literal(false)) } },
    fields: { body: field.string(), pinned: field.boolean() },
    methods: { create: method.create("write"), update: method.update("write") },
});
// queued calls and undo steps record their release; the server converts calls of earlier releases, a later release is refused
```

## Handlers

`handle` implements methods on both client and server, or on the server alone.

```ts
export const note = base.note.handle({
    archive: (call) => call.revise({ archivedAt: call.now }),
    update: async (call, next) => {
        const row = await next();
        await index(call.database, row);

        return row;
    },
    tidy: (call) => call.invoke(book, "tidy", { where: { shelf: call.target!.name }, isTidy: true }),
});
```

## Effects

`prepare` runs before the transaction, `effect` runs in it, and `settle` runs at least once after it.

```ts
export const repository = base.repository.handle({
    refresh: {
        prepare: (call) => storage.references(call.target!.id),
        effect: (call) => upsertReferences(call, call.prepared as GitReference[]),
        settle: async (call, prepared, isCommitted) => release(prepared, isCommitted),
    },
});
```

## Aggregates

An aggregate field counts or sums the objects nested in its holder.

```ts
export const notebook = defineObject({ ..., fields: { noteCount: field.count() } });
export const note = defineObject({
    ...,
    nested: { in: notebook, receive: "edit" },
    aggregates: { noteCount: { function: "count", where: { deletionRequestedAt: null } } },
});
```

## Attachments

An attachment nests in any parent type that lists it.

```ts
export const comment = defineObject({
    ...,
    nested: { in: "any", receive: "comment" },
    permissions: { read: through("parent", "read"), write: through("parent", "comment") },
});
export const note = defineObject({ ..., attachments: [comment.attach({ by: "edit" })] });
```

## Storage

An ephemeral object lives in server memory until `linger` after its client disconnects.

```ts
export const cursor = defineObject({
    ...,
    storage: "ephemeral",
    linger: { seconds: 10 },
    nested: { in: "any", receive: "present" },
});
```

A durable object type's `tables` hold its own tables and `serverTables`, and its `tier` keeps it out of databases of other tiers.

```ts
export const account = defineObject({ ..., tier: "global" });
export const accountTables: readonly Table[] = [...account.tables, accountJournal];
```

## Servers

An `ObjectServer` serves the procedures of its object types and runs their controllers.

```ts
import { ObjectServer } from "@destack/object/server";

const server = new ObjectServer({ objects: { notebook, note }, database, context, journal, audit });
const router = server.router();
await new ControlLoop(database, server.controllers(), { report }).run(signal);
await server.executeAsSystem(upload, "finish", calls, Date.now());
```

## Controllers

A type's `controller` reconciles its pending objects by key as the system, and the object server runs it.

```ts
export const reminder = defineObject({ ..., controller: {
    pending: Condition.missing("sentAt"),
    key: (row) => ({ topic: row.topic }),
    async reconcile({ rows, now, execute }) {
        await execute("send", rows.filter((row) => row.dueAt <= now));
        return nextDue(rows, now);   // the wait until the next look, or undefined
    },
} });
```

## Copies

`addressed.accept` writes the copies a home receives through `INBOX`.

```ts
const inbox: Destination<Copy> = { ...INBOX, batch: 100, accept: (copies) => addressed.accept(home, copies, homeOf) };
```

## Clients

An `ObjectClient` holds a scope's objects locally, reaches their package's service at an endpoint, and confirms local mutations from the server.

```ts
import { ObjectClient } from "@destack/object/client";

const client = await ObjectClient.open({
    database,
    objects: [notebook, note],
    scope,
    caller,
    endpoint: { url: `${origin}/.destack/service` },
    reconnect: (cell) => ({ url: serviceUrl(cell) }),   // the cell a moved scope answers at
    push: { mutations: 100 },                            // at most the server's 100
});
void client.run(signal, report);

const created = client.mutate(note).create({ title: "Ideas" });
await created.predicted;
await created.confirmed;
```

## Queries

`subscribe` returns a live query over an object type's rows, includes and aggregates.

```ts
const books = client.subscribe(notebook, {
    order: [{ column: "name", direction: "asc" }],
    include: {
        notes: { order: [{ column: "title", direction: "asc" }], limit: 5 },
        size: { via: "notes", aggregate: { values: { notes: { function: "count" } } } },
    },
});
for await (const rows of books.watch(signal)) {
    render(rows);
}
```

## Undo and branches

`undo` and `redo` apply inverse mutations, and `merge` sends a branch's mutations.

```ts
await client.undo().predicted;
client.redo();

await client.checkout("draft");
client.mutate(page).update({ id, title: "Plan B" });
await client.merge("draft");
```

## Browser tabs

A `BrowserTab` shares one SQLite database across all tabs of an origin.

```ts
const tab = await BrowserTab.open({ name: "notes", objects: [notebook, note], scope, caller, service, reconnect, report });
await tab.ready;
const notes = tab.client.subscribe(note);
```

## Stacks

`Reconciliation` writes a stack's declared records and deletes undeclared ones.

```ts
const notes = defineReconciler(note, { values: (_name, declared) => ({ title: declared.title }) });
const { changes, waiting } = await Reconciliation.apply({ database, reconcilers: [notes], manager, scope: spaceId, document });
```

## Claims

A server with a `directory` claims the keys of each unique index across a scope with every write, so the keys stay unique across databases.

```ts
export const repository = defineObject({ ..., indexes: { name: { on: ["name"], unique: true, across: account } } });
const server = new ObjectServer({ objects: { repository }, database, context, journal, audit, directory });
const found = await repository.lookup(directory, "name", ["notes"], accountId);
```
