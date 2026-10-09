# @destack/object

`defineObject` declares an object type: a "higher order" table with access rules, methods, audit, history and branching built in, whose rows sync to clients automatically (with three-shaped joins!).

```ts
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    fields,
    permissions,
    methods,
});
// serves it over one database
const store = new ObjectStore({ objects: { page }, principal: space, database, callKey, origin });
const [created] = await store.mutate(caller, {
    id: requestId,
    calls: [page.call("create", { spaceId, title: "Ideas" })],
});
const listed = client.query.page.findMany().subscribe(); // a client's live local copy
```

## Objects

`defineObject` declares a type inside its `scope`, the object type whose rows contain its rows, and `handle` implements its methods: a handler in the type's own module also runs on clients as a prediction, one in a server module runs on the server alone.

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
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
        archive: method.mutation({ permission: "edit" }),
        publish: method.mutation({
            permission: "edit",
            prepared: schema.object({ url: schema.string() }),
        }),
    }),
}).handle({
    archive: (call) => call.update({ archivedAt: call.now }),
});

// server/page.ts: prepare runs outside the transaction, so clients never predict publish
export const servedPage = page.handle({
    publish: {
        prepare: (call) => cdn.publish(call.requireTarget()),
        handler: (call) => call.update({ url: call.prepared.url }),
    },
});

// a client predicts archive at once and confirms it once the server's changes arrive
const { predicted, confirmed } = client.mutate(page).archive({ id });
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
});
```

## Controllers

`control` adds a controller that works through a type's pending rows and says when to look again.

```ts
export const reminder = base.reminder.control({
    pending: { sentAt: { isNull: true } },
    key: (row) => ({ topic: row.topic }),
    watches: [{ table: topic.table, keys: (row) => [{ topic: row.id }] }],
    async reconcile({ rows, now, mutate }) {
        const due = rows.filter((row) => row.dueAt <= now);
        await mutate(due.map((row) => base.reminder.call("send", base.reminder.target(row))));

        return nextDue(rows, now);
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

## Stores

`ObjectStore` serves object types over one database: `query` reads, and `mutate` runs calls in one transaction, once per id.

```ts
import { ObjectStore } from "@destack/object/server";

const store = new ObjectStore({ objects: { page }, principal: space, database, callKey, origin });
const row = await store.query(caller, page.call("get", { spaceId, id }));
const [renamed] = await store.mutate(caller, { id: requestId, calls: [rename] });
// the store's own principal
await store.mutate(Caller.of(space, Date.now()), { id: RequestId.create(), calls: [create] });
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
});
const store = new ObjectStore({
    objects: { message },
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

`call.mutate` runs calls inside a handler's transaction, as the same caller or as the service.

```ts
export const shelf = base.shelf.handle({
    lend: async (call) => {
        const [slip] = await call.mutate(call.service, [
            base.slip.call("issue", { shelf: call.target.name }),
        ]);

        return { slip: slip.id };
    },
});
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
await client.can(page, id, "edit");
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
