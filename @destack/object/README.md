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
    controlled: { approval: true }, // the desired generation, conditions, deletion request, approval threshold and approved plan
    bindable: true, // the consumer relation of the installations whose deployments capture the objects
    provisioned: { kind }, // a resource kind's specification, desired states, placement and provider, declarable, controlled and bindable
    declarable: { schema }, // the stack declaration managing the record
    detachable: { by: "manage" }, // detach
    shareable: { isPublic: true }, // owner, editor, commenter and viewer roles, relationships, invitations and explain
    suspendable: { by: "manage" }, // suspend and resume on a scope type
    tracked: { by: "edit", activity, session: { minutes: 10 } }, // changes grouped into activities, with history and revert
    audited: { reads: true }, // an audit event for each read
});
```

## Methods

`methods` declares each method with the permission it requires.

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

`moved` keeps the column of a renamed field.

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

`prepare` runs before a method's transaction, and `handler` runs inside it.

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

`commit` and `rollback` run after the transaction with the value `prepare` returned.

```ts
commit: async (call, prepared) => storage.erase(prepared, { idempotencyKey: call.idempotencyKey }),
```

## Aggregates

Aggregate fields such as `field.count` measure the objects nested in their holder.

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

`tables` lists the tables of a durable type for a database to declare.

```ts
export const accountTables: readonly Table[] = [...account.tables, accountJournal];
export const database = defineDatabase({ name: "main", tables: [note], copies: accountTables });
```

## Servers

`ObjectServer` serves the procedures of its object types and runs their controllers.

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

### System requests

A `SystemCall` naming a `requestId` runs once: a repeat answers the result the journal recorded, and other input under the same identifier fails with `CONFLICT`.

```ts
const requestId = await RequestId.derive(call.execution.startedAt, `${call.execution.id} ${endpointId}`);
await server.executeAsSystem(message, "create", [{ scope, requestId, input }], Date.now()); // twice, one message
```

### Capped storage

`Scope.cap` makes an installation's server refuse a caller's mutating calls but deletes and purges with `QUOTA_EXCEEDED` (402).

```ts
await Scope.cap(database, spaceId, Date.now()); // reads, lists, deletes, purges and system calls keep running
await Scope.cap(database, spaceId, null); // lift the cap
```

## Controllers

`control` sets the controller that reconciles a type's pending objects.

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

`mode: "follow"` keeps a controller running for each key while its objects are pending.

```ts
export const host = base.host.control({
    pending: { status: "enrolled" },
    mode: "follow",
    reconcile: async ({ rows, signal }) => (await serve(rows[0], signal), undefined),
});
```

## Resources

`provisioned: { kind }` declares a type's objects as resources of a kind, declarable as a `ResourceDefinition` under a stack's `resources`.

```ts
export const bucket = defineObject({
    name: "bucket",
    plural: "buckets",
    scope: space,
    provisioned: { kind: BucketKind },
    fields: {},
});

const server = new ObjectServer({
    objects: { bucket },
    database,
    callKey,
    origin,
    provisioned: { providers: [bucketProvider(host)], machine: machineId },
});
```

### Specifications

`specify` sets the desired `states` and `approval` threshold of a resource.

```ts
await client.bucket.specify({
    spaceId,
    id,
    states,
    drainingStates,
    approval: "backward-incompatible",
});
```

## Subscriber

`subscriber` lists the subscriptions that keep a server's copies current.

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

`represent` admits a follower as the principal its copied object stands for, and as the principals inside it, such as a space's installations.

```ts
export const zone = defineObject({ ..., permissions: { represent: relation("cell") } });
export const account = defineObject({ ..., permissions: { replicate: relation("machine") } });
new ObjectServer({ ..., policies: [zone], standing: [zone] });
```

## Copied types

`database.copies` lists the types a database copies from their owning service.

```ts
const subscriptions = await server.source.subscriptions(spaceId, { isHome: false }); // the chain, and the universe's rows the space reads
const isCopied = database.copies(account.table);
```

## Copy sources

`sources` lists the servers in this process that serve copied types.

```ts
const buckets = new ObjectServer({ objects: { bucket }, policies: [space], database, callKey, origin });
const spaces = new ObjectServer({ ..., policies: [bucket], sources: [buckets] });
await spaces.change({ database, scope: spaceId, now }, bucket, "create", { id, name, ...definition });
```

### Declared records

`apply` writes a stack's declared record, and `bind` binds objects to an installation.

```ts
await objects.change({ database, scope: spaceId, now }, bucket, "apply", { id, manager, values });
await objects.change({ database, scope: spaceId, now }, bucket, "bind", { installationId, ids });
```

## Scope permissions

`through` reads a permission of the scope an object lives in.

```ts
export const profile = defineObject({ ..., scope: person, permissions: { read: through("person", "read") } });
```

## Fields

`guard` requires a permission to read a field.

```ts
email: field.string().guard({ read: "update" }),
```

## Clients

`ObjectClient` keeps a scope's objects locally and calls their service.

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

## Remote clients

`RemoteClient` calls, lists and watches an installation's objects by the descriptions its manifest declares, without their object types.

```ts
const remote = new RemoteClient({
    package: release, // the installed release every call is made against
    objects: [RemoteObject.of(description, states)], // a description with its table, from the release's declared states
    scope: spaceId,
    endpoint: { url: `${origin}/.destack/service`, fetch },
    database, // the local copies of watched rows
});

await remote.call("task", "complete", { id }); // validated by the method's JSON Schema
const open = await remote.list("task", { where: { status: "open" }, limit: 50 });
for await (const rows of remote.watch("task", { orderBy: { due: "asc" } }, signal)) {
    render(rows);
}
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

`can` checks the caller's permissions against the copied access rows.

```ts
if (await client.can(note, id, "edit")) {
    showEditor();
}
```

## Typed views

`of` reads and changes a subset of a client's object types.

```ts
const { query, mutate } = client.of({ notebook, note });
await mutate.note.update({ id, title: "Groceries" });
```

## Queries

`client.query.<key>` reads once when awaited and stays live when subscribed.

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

`union` follows several queries as one ordered list.

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

`branch` lets a client edit a checked-out branch live.

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

`branch` and `at` read a branch, a log position, or both.

```ts
await client.read(note).list({ branch: id });
await client.read(note).get({ id: noteId, at: position }); // readable then and now
```

## Diffs

`diff` lists the changes of a branch, one per object.

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

`Stack.apply` writes a stack's declared records and retires undeclared ones.

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

`directory` claims the keys of each unique index, and `lookup` finds the object holding a key.

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

## Errors

An invalid declaration throws an `ObjectError`.

```ts
import { ObjectError } from "@destack/object";

new ObjectError("CYCLIC_DECLARATION", "note contains itself").toServiceError(); // { code: "INTERNAL_SERVER_ERROR", … }
```
