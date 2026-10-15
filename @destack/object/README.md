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
}).methods((method) => ({ get: method.get("read"), create: method.create("edit") }));
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
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    fields: {
        title: field.string(schema.string().max(500)).default(""),
        archivedAt: field.time().optional(),
        url: field.string(schema.string()).optional(),
    },
    permissions: { read: relation("viewer"), edit: relation("editor") },
})
    .methods((method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
        archive: method.mutation({ permission: "edit" }),
        publish: method.mutation({
            permission: "edit",
            prepared: schema.object({ url: schema.string() }),
        }),
    }))
    .implement({
        methods: { archive: (call) => call.update({ archivedAt: call.now }) },
    });

// server/page.ts: prepare runs outside the transaction, so clients never predict publish
export const servedPage = page.implement({
    methods: {
        publish: {
            prepare: (call) => cdn.publish(call.requireTarget()),
            handle: (call) => call.update({ url: call.prepared.url }),
        },
    },
});

// a client predicts archive at once and confirms it once the server's changes arrive
const { predicted, confirmed } = client.mutate(page).archive({ id });
```

## References

`field.reference` keeps an object's identifier as `<name>Id`, while `field.object` and `field.subject` keep a reference to any object or principal as `<name>PackageId`, `<name>Type`, `<name>Scope` and `<name>Id`.

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
    recoverable: { within: { days: 30 }, by: "edit" }, // delete, restore and purge
    controlled: { approval: true }, // desired generation, conditions and approved plan
    provisioned: { kind }, // a resource kind's specification and provider
    shareable: { isPublic: true }, // owner, editor, commenter and viewer roles
    tracked: { by: "edit", activity, session: { minutes: 10 } }, // history and revert
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

`type.call` makes a call of a method as a value, and `type.target` names the row it acts on.

```ts
const rename = page.call("rename", { ...page.target(row), title: "Plans" });
const create = page.call("create", { spaceId, title: "Ideas" });
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
// the store's own principal
await store.mutate(store.self, { id: RequestId.create(), calls: [create] });
```

## Events

A type kept in the event log takes `storage: { kind: "events", … }`: `create` and `createMany` append its rows under the caller's `id` and `time`, internally or under a permission, the same `get` and `list` read them hot and flushed alike, also from a durable method's handler, and `series` folds them; a guarded field masks the personal values its schema marks.

```ts
export const call = defineObject({
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
        data: field.json(AuditCall).guard({ read: "unmask" }),
        actor: field.string(),
        method: field.string(),
    },
    permissions: ["read", "unmask", "record"],
}).methods((method) => ({
    record: method.createMany("record", { isInternal: true, fields: ["data", "actor", "method"] }),
}));
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
await store.mutate(store.self, {
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

An index declared `across` a scope keeps its names unique across databases: a store reserves a write's names before it commits and confirms them after, through the `claimMethods` a claim type declares, and `Claim.lookup` finds the object owning a name.

```ts
indexes: { handle: { on: ["handle"], unique: true, across: () => universe } },

const claim = defineObject({ name: "claim", /* … */ }).methods((method) => ({ ...claimMethods }));
const home = RemoteStore.of(() => accounts.replica, Scope.universe.id);
const store = new ObjectStore({ servers: [profileServer], claims: { object: claim, home }, principal: space, database, callKey, events, origin });
const owner = await Claim.lookup(profile, store.claims, "handle", ["ada"]); // { scope, id }
```

## Partitions

A scope type declaring `partition: true` keeps each of its objects' contents in a partition of their own, served by one instance; a store serving one partition answers a call in another with a 421 naming the partition.

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

`call.mutate` runs calls inside a handler's transaction, as the same caller or as the store itself (`call.self`).

```ts
export const shelf = base.shelf.implement({
    methods: {
        lend: async (call) => {
            const [slip] = await call.mutate(call.self, [
                base.slip.call("issue", { shelf: call.target.name }),
            ]);

            return { slip: slip.id };
        },
    },
});
```

## Descriptions

`type.describe()` writes a type's definition as data, and `ObjectType.ofAll` declares the same types from it against the declarations it refers to.

```ts
const description = page.describe(); // { package, definition: { fields: { title: { $field: [...] } } } }
const [declared] = ObjectType.ofAll([description], [space, principal.user]);
```

## Clients

`ObjectClient` keeps a scope's objects in a local database, follows queries live, and undoes, branches and diffs changes.

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
await client.check(page, id, "edit");
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
