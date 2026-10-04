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
        status: field.state({
            initial: "draft",
            transitions: { publish: { from: ["draft"], to: "published", permission: "write" } },
        }),
    },
    nested: { in: notebook, receive: "write", move: "write" },
    recoverable: { within: { days: 30 }, by: "write" },
    permissions: {
        read: union(relation("owner"), through("parent", "read")),
        write: relation("owner"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
    }),
});
```

## Traits

Each trait option of `defineObject` adds columns, permissions and methods to the type.

```ts
defineObject({
    ...definition,
    nested: { in: folder, receive: "edit" }, // the parent reference, and move
    recoverable: { within: { days: 30 }, by: "edit" }, // the trash, with delete, restore and purge
    expiring: [{ after: { days: 90 }, from: "updatedAt" }], // removal by the system once a rule's window passes
    projected: { from: () => memo, to: "recipient", source: "memoId" }, // rows of another type projected into each recipient's home
    versioned: true, // the version number within the parent
    controlled: { approval: true }, // the desired generation, conditions, deletion request and approved plan
    bindable: true, // the consumer relation of the installations whose deployments capture the objects
    provisioned: { kind }, // a resource kind's specification, placement and provider, controlled and bindable
    declarable: { schema }, // the stack declaration managing the record
    detachable: { by: "manage" }, // detach
    shareable: { isPublic: true }, // owner, editor, commenter and viewer roles, relationships, invitations and explain
    suspendable: { by: "manage" }, // suspend and resume on a scope type
    tracked: { by: "edit", activity, session: { minutes: 10 } }, // changes grouped into activities, with history and revert
    audited: { reads: true }, // an audit event for each read
});
```

## Methods

`methods` receives the builder declaring each method with the permission it requires, and `isSystem: true` keeps a method to the system.

```ts
methods: (method) => ({
    get: method.get("read"),
    list: method.list("read"),
    create: method.create("write", { fields: ["title"] }),
    update: method.update("write", { fields: ["title"] }),
    delete: method.delete("write"),
    archiveAll: method.updateMany("write", { fields: ["archivedAt"], match: ["parentId"] }),
    search: method.query({ permission: "read", input: SearchInput, output: NotePage }),
    reindex: method.mutation({ permission: null, isSystem: true }),
}),
```

## Releases

`moved` keeps a renamed field's column and earlier callers' inputs, and `convert` computes fields by the release introducing them.

```ts
const note = defineObject({
    name: "note",
    moved: { fields: { body: "text" } }, // calls of earlier releases may still send `text`
    convert: {
        "2026.10.0": {
            pinned: Expression.coalesce(Expression.column("pinned"), Expression.literal(false)),
        },
    },
    fields: { body: field.string(), pinned: field.boolean() },
    methods: (method) => ({ create: method.create("write"), update: method.update("write") }),
});
// queued calls and undo steps record their release; the server converts calls of earlier releases, a later release is refused
```

## Handlers

`handle` implements methods on both client and server, or on the server alone.

```ts
export const note = base.note.handle({
    archive: (call) => call.update({ archivedAt: call.now }),
    update: async (call, next) => {
        const row = await next();
        await index(call.database, row);

        return row;
    },
    tidy: (call) =>
        call.invoke(book).tidy({ where: { shelf: call.requireTarget().name }, isTidy: true }),
});
```

## External work

`prepare` runs before a method's transaction, `handler` in it, and `commit` or `rollback` at least once after it, as the recorded settlement says.

```ts
export const repository = base.repository.handle({
    purge: {
        prepare: async (call) => columnsOf(call.requireTarget()),
        commit: async (_call, prepared) => storage.erase(prepared),
    },
    create: {
        prepare: (call) => storage.provide(call.input),
        handler: (call, next) => next(call.with({ input: { ...call.input, ...call.prepared } })),
        rollback: async (_call, prepared) => prepared && storage.release(prepared),
    },
});
```

## Settlement

`commit` and `rollback` take the recorded call and the value `prepare` returned, parsed by the method's `prepared` schema, and pass `call.idempotencyKey` on so the external system does the work once.

```ts
commit: async (call, prepared) => storage.erase(prepared, { idempotencyKey: call.idempotencyKey }),
```

## Aggregates

`field.count` and the other aggregate fields measure the objects nested in their holder.

```ts
export const notebook = defineObject({ ..., fields: { noteCount: field.count() } });
export const note = defineObject({
    ...,
    nested: { in: notebook, receive: "edit" },
    aggregates: { noteCount: { function: "count", where: { deletionRequestedAt: null } } },
});
```

## Attachments

`nested: { in: "any" }` nests an attachment in any parent type that lists it.

```ts
export const comment = defineObject({
    ...,
    nested: { in: "any", receive: "comment" },
    permissions: { read: through("parent", "read"), write: through("parent", "comment") },
});
export const note = defineObject({ ..., attachments: [comment.attach({ by: "edit" })] });
```

## Storage

`storage: "ephemeral"` keeps objects in server memory until `linger` after their client disconnects.

```ts
export const cursor = defineObject({
    ...,
    storage: "ephemeral",
    linger: { seconds: 10 },
    nested: { in: "any", receive: "present" },
});
```

## Tables

`tables` lists a durable type's tables with `serverTables`, which a database declares as its own or as copies.

```ts
export const accountTables: readonly Table[] = [...account.tables, accountJournal];
export const database = defineDatabase({ name: "main", tables: [note], copies: accountTables });
```

## Servers

`ObjectServer` serves the procedures of its object types and runs their controllers, keeping their journal under the `callKey`.

```ts
import { ObjectServer } from "@destack/object/server";

const server = new ObjectServer({
    objects: { notebook, note },
    database,
    callKey,
    origin: { package: notesService.package, service: "notes" },
});
const router = server.router();
await new ControlLoop(database, server.controllers(), { report }).run(signal);
await server.executeAsSystem(upload, "finish", calls, Date.now());
```

## Controllers

`control` sets the controller reconciling a type's pending objects by key as the system.

```ts
export const reminder = base.reminder.control({
    pending: { sentAt: { isNull: true } },
    key: (row) => ({ topic: row.topic }),
    watches: [{ table: topic.table, keys: (row) => [{ topic: row.id }] }], // other rows selecting keys again
    async reconcile({ rows, now, execute }) {
        await execute(
            "send",
            rows.filter((row) => row.dueAt <= now),
        );
        return nextDue(rows, now); // the wait until the next look, or undefined
    },
});
```

## Following controllers

`mode: "follow"` keeps a controller's process running for each key until its objects leave the pending ones.

```ts
export const host = base.host.control({
    pending: { status: "enrolled" },
    mode: "follow",
    reconcile: async ({ rows, signal }) => (await serve(rows[0], signal), undefined),
});
```

## Subscriber

`subscriber` lists the subscriptions a server keeps its copies current through, each with its publisher, and the server drops the kept copies no subscription names any longer.

```ts
const server = new ObjectServer({
    objects: {},
    policies: [account],
    database,
    origin,
    // copy the universe's rows the workload reads, and one copy of its scopes' chains
    subscriber: Subscriber.of(publisher, () => server.source.workloadSubscriptions(placementId)),
});
```

## Copy admission

`represent` admits a follower as the principal a copied object stands for through its `self` relation where a server lists the type in `standing`, and `replicate` admits it to a chain.

```ts
export const zone = defineObject({ ..., permissions: { represent: relation("cell") } });
export const account = defineObject({ ..., permissions: { replicate: relation("host") } });
new ObjectServer({ ..., policies: [zone], standing: [zone] });
```

## Copied types

`database.copies` reads the types a database declares as copies, which a server keeps from their owning service and refuses writes to.

```ts
const subscriptions = await server.source.subscriptions(spaceId, { isHome: false }); // the chain, and the universe's rows the space reads
const isCopied = database.copies(account.table);
```

## Scope permissions

`through` reads a permission of the scope an object lives in under the scope type's name.

```ts
export const profile = defineObject({ ..., scope: person, permissions: { read: through("person", "read") } });
```

## Fields

`guard` requires a permission to read a field, which readers without it see concealed.

```ts
email: field.string().guard({ read: "update" }),
```

## Clients

`ObjectClient` keeps a scope's objects locally, calls their package's service at an endpoint, and confirms local mutations from the server.

```ts
import { ObjectClient } from "@destack/object/client";

const client = await ObjectClient.open({
    database,
    objects: { notebook, note },
    scope,
    caller,
    endpoint: { url: `${origin}/.destack/service` },
    reconnect: (cell) => ({ url: serviceUrl(cell) }), // the cell a moved scope answers at
    push: { mutations: 100 }, // at most the server's 100
});
void client.run(signal, report);

const created = client.mutate(note).create({ title: "Ideas" });
await created.predicted;
await created.confirmed;
```

## Client copies

`stream` serves a client one `Subscription` per shape.

```ts
const shapes = [
    QUERIES_SHAPE, // the live queries of its durable objects
    EPHEMERAL_SHAPE, // its ephemeral objects, written by the client
    EXTERNAL_SHAPE, // the queries of its external objects
    ACCESS_SHAPE, // the access rows its caller's own checks read
];
```

## Checks

`can` decides the caller's permissions over the copied access rows with the same policies the server compiles.

```ts
if (await client.can(note, id, "edit")) {
    showEditor();
}
```

## Typed views

`of` reads and changes some of a client's object types by key.

```ts
const { query, mutate } = client.of({ notebook, note });
await mutate.note.update({ id, title: "Groceries" });
```

## Queries

`client.query.<key>` takes db's `FindOptions`, with `deleted` selecting a recoverable root's trash, and reads once when awaited or stays live when subscribed.

```ts
const books = client.query.notebook.findMany({
    orderBy: { name: "asc" },
    with: { notes: { orderBy: { title: "asc" }, limit: 5 } },
});
for await (const rows of books.subscribe().watch(signal)) {
    render(rows);
}
const first = await client.query.note.findFirst({ where: { title: { like: "Idea%" } } });
const counts = await client.query.note.aggregate({
    groupBy: ["parentId"],
    values: { notes: { function: "count" } },
});
```

## Unions

`union` follows several queries as one list in a shared order and limit, each entry naming its member.

```ts
const activity = client.union(
    { notebooks: client.query.notebook.findMany(), notes: client.query.note.findMany() },
    { orderBy: { createdAt: "desc" }, limit: 20 },
);
```

## Undo

`undo` and `redo` apply inverse mutations.

```ts
await client.undo().predicted;
client.redo();
```

## Branches

`branch` lets a client edit a checked-out branch live, pushing its edits to the branch.

```ts
const client = await ObjectClient.open({
    database,
    objects,
    package: notesService.package,
    branch: branchType,
    scope,
    caller,
    endpoint,
    reconnect,
});
const { id } = await client.mutate(branch).create({ title: "Packing list" }).predicted;
await client.checkout(id);
client.mutate(note).create({ parentId, title: "Socks" });
await client.mutate(branch).merge({ id }).confirmed; // the device shows the main line again
```

## Branch reads

`branch` and `at` read a branch over the main line, a log position, or both.

```ts
await client.read(note).list({ branch: id });
await client.read(note).get({ id: noteId, at: position }); // readable then and now
```

## Diffs

`diff` follows a branch's changes, one per object, with the written fields that differ.

```ts
const diff = client.diff(id);
for await (const changes of diff.watch(signal)) render(changes); // [{ object, id, change, before, after, fields }]
```

## Browser tabs

`BrowserTab` shares one SQLite database across all tabs of an origin.

```ts
import { BrowserTab } from "@destack/object/browser";

const tab = await BrowserTab.open({
    name: "notes",
    objects: { notebook, note },
    scope,
    caller,
    endpoint,
    reconnect,
    report,
});
await tab.ready;
const notes = await tab.client.query.note.findMany();
```

## Stacks

`declare` sets how a stack's declarations of a type become its records, and `Stack.apply` writes a stack's declared records and retires undeclared ones.

```ts
export const note = base.note.declare({ values: (_name, declared) => ({ title: declared.title }) });
const { steps, deferred } = await Stack.apply({
    database,
    objects: [note],
    manager,
    scope: spaceId,
    document,
});
```

## Claims

`directory` claims the keys of each unique index within its `across` scope type with every write, and `lookup` finds the object owning a key.

```ts
export const repository = defineObject({
    ...,
    indexes: {
        name: { on: ["name"], unique: true, across: () => account },
        id: { on: ["id"], unique: true, across: () => universe },
    },
});
const server = new ObjectServer({ objects: { repository }, database, callKey, origin, directory });
const found = await repository.lookup(directory, "name", ["notes"], accountId);
```
