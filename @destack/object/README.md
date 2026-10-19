# @destack/object

`defineObject` declares an object type: a "higher order" table with access rules, methods, audit, history and branching built in, whose rows sync to clients automatically (with three-shaped joins!).

```ts
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    fields,
    permissions,
    constraints: { title: { kind: "index", on: ["title"] } },
}).methods(); // get and list by read; create, update and delete by write
// serves its package over one database
const store = new ObjectStore({
    servers: [defineServer(pages, { objects: [page] })],
    principal: space,
    database,
    callKey,
    events,
    origin,
});
const [created] = await store.mutate(caller, {
    id: requestId,
    calls: [page.call("create", { spaceId, title: "Ideas" })],
});
const listed = client.query.page.findMany().subscribe(); // a client's live local replica
```

## Objects

`defineObject` declares a type inside its `scope`, the object type whose rows contain its rows, `.methods` completes it, and `implement` attaches its code (method handlers, a controller, a declaration handler): a handler in the type's own module also runs on clients as a prediction, one in a server module runs on the server alone.

```ts
import { relation } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";

// page.ts, which clients and the server import
export const page = defineObject(
    {
        name: "page",
        plural: "pages",
        scope: space,
        fields: {
            title: field.string(schema.string().max(500)).default(""),
            archivedAt: field.time().optional(),
            url: field.string(schema.string()).optional(),
            template: field.string().immutable(),
        },
        permissions: { read: relation("viewer"), write: relation("editor") },
    },
    (method) => ({
        delete: false,
        archive: method.mutation({ permission: "write" }),
        publish: method.mutation({
            permission: "write",
            prepared: schema.object({ url: schema.string() }),
        }),
    }),
).implement({
    methods: { archive: (context) => context.update({ archivedAt: context.now }) },
});

// server/page.ts: prepare runs outside the transaction, so clients never predict publish
export const servedPage = page.implement({
    methods: {
        publish: {
            prepare: (context) => cdn.publish(context.target),
            handle: (context) => context.update({ url: context.prepared.url }),
        },
    },
});

// a client predicts archive at once and confirms it once the server's changes arrive
const { predicted, confirmed } = client.mutate(page).archive({ id });
```

## Methods

Every type derives `get` and `list` under `read`, and a type with a `write` permission derives `create`, `update` and `delete` under it: a declared method replaces its derived one, `false` removes one, and an `immutable` field is written at creation only.

```ts
export const label = defineObject(
    {
        name: "label",
        plural: "labels",
        scope: space,
        fields: { name: field.string(), color: field.string().immutable() },
        permissions: { read: relation("viewer"), write: relation("editor") },
    },
    () => ({ delete: false }), // get, list, create, update; update leaves color out
);
```

## References

`field.reference` stores an object's identifier as `<name>Id`, while `field.object` and `field.subject` store a reference to any object or principal as `<name>PackageId`, `<name>Type`, `<name>Scope` and `<name>Id`.

```ts
fields: {
    folder: field.reference(folder, { delete: "cascade" }), // a foreign key in the space's database
    parent: field.reference((): ObjectOf<{ identity: "page" }> => page), // a type declared later
    owner: field.reference(principal.user), // a global principal's identifier
    origin: field.object(page), // originPackageId, originType, originScope, originId
    author: field.subject(principal.user, principal.agent), // authorPackageId … authorId
},
```

```ts
await notes.create({ ...ObjectReference.columns("origin", page.reference(scope, id)), title });
const origin = ObjectReference.at("origin", row);
```

## Traits

Each trait option of `defineObject` adds columns, permissions and methods to the type.

```ts
defineObject({
    ...definition,
    recoverable: { within: { days: 30 }, by: "write" }, // delete, restore and purge
    controlled: { approval: true }, // desired generation, conditions and approved plan
    provisioned: { kind }, // a resource kind's specification and provider
    shareable: { isPublic: true }, // owner, editor, commenter and viewer roles
    tracked: { by: "write", activity, session: { minutes: 10 } }, // history and revert
    orderable: true, // a fractional index among the siblings
    search: { searchable: ["title"], filterable: ["status"], sortable: ["dueAt"] },
}).methods(build);
```

## Controllers

A type's `controller` works through its pending rows and says when to look again.

```ts
export const reminder = base.reminder.implement({
    controller: {
        pending: { sentAt: { isNull: true } },
        key: (row) => ({ topic: row.topic }),
        watches: [{ table: topic.table, keys: (row) => [{ topic: row.id }] }],
        async reconcile({ rows, now, mutate }) {
            const due = rows.filter((row) => row.dueAt <= now);
            await mutate(due.map((row) => base.reminder.call("send", base.reminder.target(row))));

            return nextDue(rows, now);
        },
    },
});
export const reminderConditions = new Conditions(["Sent"]);
```

## Calls

`type.call` makes a call of a method as a value, and `type.target` points it at the row it acts on.

```ts
const rename = page.call("rename", { ...page.target(row), title: "Plans" });
const create = page.call("create", { spaceId, title: "Ideas" });
```

## Contexts

A handler gets the `MethodContext` its method's declaration types: `input`, `target`, `caller`, and `read` and `mutate` in the call's transaction.

```ts
const rename = method.mutation({
    permission: "write",
    input: schema.object({ title: schema.string() }),
});
page.implement({
    methods: {
        rename: (context) => context.update({ title: context.input.title }), // target: the loaded page
    },
});
```

## Servers

`defineServer` is the one way to serve a package: its handled types, controllers, providers, follows, external files and adapters; `at` serves it as an installation inside a store of other packages.

```ts
import { defineServer } from "@destack/object/server";

export default defineServer(pages, { objects: [page.implement({ methods })], controllers: [] });
const vault = vaultServer.at(installation); // served as its installation in a space's store
```

## Stores

`ObjectStore` serves package servers over one database: `query` reads, `mutate` runs calls in one transaction, once per id, and `packages` answers each package below its mount.

```ts
import { ObjectStore } from "@destack/object/server";

const store = new ObjectStore({
    servers: [defineServer(pages, { objects: [page] })],
    principal: space,
    database,
    callKey,
    events,
    origin,
});
const row = await store.query(caller, page.call("get", { spaceId, id }));
const [renamed] = await store.mutate(caller, { id: requestId, calls: [rename] });
// the installation serving the type, else the store's own principal
await store.mutate(store.serving(page), { id: RequestId.create(), calls: [create] });
```

## Replicas

A server follows a `defineShape` at fixed keys, at one key per row of its own, or on demand, and changes replicated rows at their home over the store's `Uplink`; every store also follows its scope's chain and, from the universe, the access grants delegated into its scope.

```ts
import { defineShape, Follow } from "@destack/object";

const cards = defineShape({
    name: "cards",
    parameters: schema.object({ title: schema.string() }),
    objects: [card],
    queries: ({ title }) => [card.query({ where: { title } })],
});
const usable = defineShape({
    name: "connections",
    parameters: schema.object({}),
    objects: [connection],
    queries: () => [connection.query({ permission: "use" })], // only the connections the follower may use
});
const directory = Follow.of(directoryShape, { keys: [{ key: { scope: Scope.universe.id } }] });
const activities = relationship.follow(activityShape, { key: keyOf });
const byTitle = Follow.of(cards, { onDemand: { ttl: { minutes: 10 } } });

await store.acquire(byTitle, { key: { scope: spaceId, title: "Milk" } }, signal); // replicated once this returns
```

## Events

A type stored in the event log takes `storage: { kind: "events", … }`: `create` and `createMany` append its rows under the caller's `id` and `time`, internally or under a permission, the same `get` and `list` read them hot and flushed alike, also from a durable method's handler, and `series` folds them; a guarded field masks the personal values its schema marks.

```ts
export const call = defineObject(
    {
        name: "call",
        plural: "calls",
        storage: {
            kind: "events",
            flush: { maxAge: { hours: 1 }, maxRows: 100_000 },
            retention: { days: 400 },
            delivery: "exactly-once",
            isLocked: true,
            subject: "actor",
        },
        fields: {
            data: field.json(CallEvent).guard({ read: "unmask" }),
            actor: field.string(),
            method: field.string(),
        },
        permissions: ["read", "unmask", "record"],
    },
    (method) => ({
        record: method.createMany("record", {
            isInternal: true,
            fields: ["data", "actor", "method"],
        }),
    }),
);
const events = new EventStore({
    database,
    kinds: [call.eventKind()],
    files,
    personal,
    clock,
    report,
});
const store = new ObjectStore({
    servers: [objectServer],
    principal: space,
    database,
    callKey,
    events,
    origin,
});
await store.mutate(store.serving(call), {
    id: requestId,
    calls: [call.call("record", { scope, items: [{ id, time, data, actor, method }] })],
});
await store.query(
    caller,
    call.call("list", { scope, where: { outcome: "failure" }, orderBy: { time: "desc" } }),
);
await store.query(
    caller,
    call.call("series", { scope, fold: "count", group: ["method"], from, before }),
);
```

## Call log

The journal appends each audited call to the `call` events type, which object's own server serves wherever a store composes it.

```ts
import objectPackage from "@destack/object/package";
import objectServer, { ObjectStore } from "@destack/object/server";

const store = new ObjectStore({ servers: [objectServer, ...servers], ...options });
const calls = createClient(objectPackage, endpoint);
await calls.call.list({ scope, where: { outcome: "failure" }, orderBy: { time: "desc" } });
```

## Unique names

An index declared `across` a scope holds its names unique across databases: a store reserves a write's names before it commits and confirms them after, through the `claimMethods` a claim type declares, and `Claim.lookup` finds the object owning a name.

```ts
indexes: { handle: { on: ["handle"], unique: true, across: () => universe } },

const claim = defineObject({ name: "claim", /* … */ }, () => claimMethods);
const home = RemoteStore.of(accounts.store, Scope.universe.id);
const store = new ObjectStore({ servers: [profileServer], claims: { object: claim, home }, principal: space, database, callKey, events, origin });
const owner = await Claim.lookup(profile, store.claims, "handle", ["ada"]); // { scope, id }
```

## Partitions

A scope type declaring `partition: true` stores each of its objects' contents in a partition of their own, served by one instance; a store serving one partition answers a call in another with a 421 naming the partition.

```ts
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: space,
    isScope: true,
    partition: true,
}).methods();
const store = new ObjectStore({
    servers: [defineServer(rooms, { objects: [message] })],
    partition: roomId,
    database,
    principal,
    callKey,
    origin,
});
// a call in another room: 421 MISDIRECTED_REQUEST { scope, partition }, which the router resends once
request.headers.set(PARTITION_HEADER, roomId);
```

## Nested calls

`context.mutate` runs calls inside a handler's transaction, as the same caller or as the code itself (`context.self`: the installation serving the type, else the store's principal); `context.space()` acts for the space, as its installation under the space's grant; `context.enabled("push")` reads whether it has that capability.

```ts
export const shelf = base.shelf.implement({
    methods: {
        lend: async (context) => {
            const [slip] = await context.mutate(context.self, [
                base.slip.call("issue", { shelf: context.target.name }),
            ]);

            return { slip: slip.id };
        },
    },
});
const recipient = await context.query(await context.space(), user.call("recipient", { id }));
if (context.enabled("push")) await deliverPush(recipient);
```

## Descriptions

`type.describe()` writes a type's definition as data, and `ObjectType.ofAll` declares the same types from it against the declarations it refers to.

```ts
const description = page.describe(); // { package, definition: { fields: { title: { $field: [...] } } } }
const [declared] = ObjectType.ofAll([description], [space, principal.user]);
```

## Clients

`ObjectClient` replicates a scope's objects in a local database, follows queries live, and undoes, branches and diffs changes.

```ts
import { ObjectClient } from "@destack/object/client";

const client = await ObjectClient.open({
    database,
    objects: { page },
    scope,
    caller,
    endpoint,
    reconnect,
});
for await (const rows of client.query.page
    .findMany({ orderBy: { position: "asc" } })
    .subscribe()
    .watch(signal)) {
    render(rows);
}
await client.undo().predicted;
await client.check(page, id, "write");
```

## Browser tabs

`BrowserTab` shares one SQLite database across the tabs of an origin.

```ts
import { BrowserTab } from "@destack/object/browser";

const tab = await BrowserTab.open({
    name: "pages",
    objects: { page },
    scope,
    caller,
    client,
    endpoint,
    reconnect,
    report,
});
await tab.client.query.page.findMany();
```
