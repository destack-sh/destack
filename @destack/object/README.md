# @destack/object

`defineObject` declares an object type: a table with access rules, methods and history, whose rows sync to clients.

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
    servers: [implementPackage(pages, { objects: [page] })],
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

`defineObject` declares a type and `implement` gives it code, which also runs on clients unless it sits in a server module.

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

Every type gets `get` and `list`, a type with a `write` permission also gets `create`, `update` and `delete`, and `false` removes one.

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

## Custom methods

A type should only add its own method to change its object in a way `update` cannot, to follow another standard's protocol, or to run a step of its controller.

```ts
(method) => ({
    cancel: method.mutation({ permission: "cancel" }), // change the object
    token: method.mutation({ permission: "token", output: Token }), // follow OAuth 2.0
    follow: method.mutation({ permission: "follow", isInternal: true }), // run the controller's step
}),
// read and write everything else through the standard methods of the type keeping the state
// sign up through user.create, invite through invitation.create
// compute derived values where they are read
```

## References

`field.reference` stores one type's identifier as `<name>Id`, and `field.object` or `field.subject` store a reference to any object or principal as `<name>PackageId`, `<name>Type`, `<name>Scope` and `<name>Id`.

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

A trait adds columns, permissions and methods to a type.

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

## Declarations

A declarable type takes stack declarations as its create input: the key fills a field from the declaration's name, and references take declared names.

```ts
const badge = defineObject({
    ...definition,
    declarable: { from: "create", key: "label", references: { tagId: tag, holder: "subject" } },
});

const badges: Record<string, StackInputOf<typeof badge>> = {
    open: { tag: "red", holder: { anyone: true } }, // a declared tag, every caller
    personal: { tag: { id: tagId }, holder: { id: userId } }, // an existing tag, a global principal
    staff: { tag: "red", holder: { id: groupId, scope: "enclosing", relation: "member" } },
    owners: { tag: "red", holder: { scope: "enclosing", relation: "owner" } },
};
```

## Controllers

A type's `controller` works through its pending rows, and says when to look again.

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

`type.call` builds a method call as a value, and `type.target` names the row it acts on.

```ts
const rename = page.call("rename", { ...page.target(row), title: "Plans" });
const create = page.call("create", { spaceId, title: "Ideas" });
```

## Contexts

A handler gets a `MethodContext` with the call's `input`, `target` and `caller`, and reads and writes in the call's transaction.

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

## Implementations

`implementPackage` implements a package's types, controllers and follows, and `at` implements it as one installation in a store.

```ts
import { implementPackage } from "@destack/object/server";

export default implementPackage(pages, { objects: [page.implement({ methods })], controllers: [] });
const vault = vaultServer.at(installation); // served as its installation in a space's store
```

## Servers

`Server.start` hosts a package's handler over HTTP with its health, authentication and controllers, and `implementHealth` answers its health procedures.

```ts
import { Health, Server } from "@destack/object/server";

const server = Server.start({
    ...handler, // the routes and enforcement a store builds for a package
    clock,
    health: new Health(handler.package.name),
    authenticate: (request) => verifier.authenticate(request, clock.now()),
    authorizeMachine: async () => {},
});
await server.fetch(request);
```

## Stores

`ObjectStore` serves packages over one database: `query` reads, and `mutate` runs calls in one transaction.

```ts
import { ObjectStore } from "@destack/object/server";

const store = new ObjectStore({
    servers: [implementPackage(pages, { objects: [page] })],
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

A server follows a `defineShape` to keep a copy of rows that live in another store, and every store also follows its scope's chain and the universe's grants into it.

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

## Watermarks

`wait` waits until a store has caught up to a position in another store's log, and asks that store when it falls behind.

```ts
await store.wait({ scope: homeId, epoch, sequence }, signal); // asks the source only while behind
```

## Events

A type with `storage: { kind: "events" }` keeps its rows in the event log, where `create` appends and `series` folds them.

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

The journal records each audited call as a `call` event.

```ts
import objectPackage from "@destack/object/package";
import objectServer, { ObjectStore } from "@destack/object/server";

const store = new ObjectStore({ servers: [objectServer, ...servers], ...options });
const calls = createClient(objectPackage, endpoint);
await calls.call.list({ scope, where: { outcome: "failure" }, orderBy: { time: "desc" } });
```

## Unique names

An index declared `across` a scope keeps its names unique across databases, and `Claim.lookup` finds a name's owner.

```ts
indexes: { handle: { on: ["handle"], unique: true, across: () => universe } },

const claim = defineObject({ name: "claim", /* … */ }, () => claimMethods);
const home = RemoteStore.of(accounts.store, Scope.universe.id);
const store = new ObjectStore({ servers: [profileServer], claims: { object: claim, home }, principal: space, database, callKey, events, origin });
const owner = await Claim.lookup(profile, store.claims, "handle", ["ada"]); // { scope, id }
```

## Partitions

A scope type with `partition: true` keeps each object's contents in its own partition, served by one instance, and a store answers a call for another partition with a 421 that names it.

```ts
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: space,
    isScope: true,
    partition: true,
}).methods();
const store = new ObjectStore({
    servers: [implementPackage(rooms, { objects: [message] })],
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

`context.mutate` runs more calls in a handler's transaction as the caller or the code itself (`context.self`), `context.space()` acts for the space, and `context.enabled` reads a capability.

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

`ObjectDescription.of` writes a type's definition as data, and `ObjectTypePool.declare` reads it back.

```ts
const description = ObjectDescription.of(page); // { package, definition: { fields: { title: { $field: [...] } } } }
const [declared] = ObjectTypePool.of([space, principal.user]).declare([description]);
```

## Packages

`definePackage` declares a package's handle from its objects, views, adapters, resources, secrets, the permissions its installations request of their space and their users, and its capabilities, and `defineAdapter` answers a protocol others define below the package's mount.

```ts
export const npm = defineAdapter("npm", { path: "/npm" });
export const hooks = defineAdapter("hooks", { path: "/hooks", callers: [principal.installation] });
export const sync = defineAdapter("sync", { path: "/sync", callers: "self" }); // the installation itself
export default definePackage({
    objects: { repository, release },
    adapters: { npm, hooks },
    permissions: {
        installation: {
            space: [message.permission("send")],
            universe: [user.permission("recipient")],
        },
        user: { home: [notification.permission("read")] },
    },
    capabilities: {
        push: {
            reason: "Push notifications",
            optional: true, // asked for once its code needs it
            permissions: { installation: { universe: [client.permission("unsubscribe")] } },
            features: [{ packageId: pricing, name: "push" }], // the account's plan must include it
        },
    },
    since: "2026.8.0",
});
```

## Procedures

`defineProcedure` declares a procedure with its authentication and permission, as oRPC's `oc`, and `CallError` is the error a caller receives, as `ORPCError`.

```ts
const list = defineProcedure({
    package: Package.of(pages), // the package whose releases its callers speak
    authentication: "identity",
    permission: page.permission("read"),
    audit: false,
});
list.route({ method: "GET", path: "/pages" }).output(PageList);
throw new CallError("NOT_FOUND", { message: `no page ${id}` });
```

## Workloads

`defineWorkload` declares a `start` that runs once per instance and returns the served packages and what it does with its host's webhook deliveries and the WebSockets its host hibernates, as Durable Objects do.

```ts
export const workload = defineWorkload({
    name: "main",
    start: async (context) => ({
        packages: [implementPages(context)],
        webhooks: { fetch: receive },
    }),
});

// a room's partition: accept a socket in a route, answer its messages through the host
start: async ({ server, hibernation }) => ({
    packages: [
        { package: rooms, router: {}, fetch: async () => hibernation.accept({ id, attachment }) },
    ],
    sockets: { message: async (socket, bytes) => socket.send(bytes), close: async () => {} },
});
server.partition; // the one partition the instance serves, from the Destack-Partition header
server.at(Egress.address(alias, spaceId)); // another installation by its alias, through the egress
await server.keys.derivePrivateKey("vapid"); // a key of the package's own, rooted in its space's identity
request.headers.get(FORWARDED_URL_HEADER); // a route's request at ROUTE_PATH, handed to the workload's own package
await instance.fetch(rooms, request); // a request to one served package
await instance.reload(); // serve the next start once started, drain the previous beside it, fold reloads meanwhile
{ package: rooms, router: {}, controllers, activation }; // run its controllers once activation settles
```

## Bindings

`defineInstallationBinding` declares a dependency on an installation of another package, and `defineHttps` on an external HTTPS API, both called through the host's egress.

```ts
export const pages = defineInstallationBinding("pages", pagesPackage);
export const github = defineHttps("github", { url: "https://api.github.com" });

const result = await safe(pages.get(context.resources).page.update(input)); // oRPC's safe
const response = await github.get(context.resources).fetch("/repos/acme/app/issues");
```

## Clients

`createClient` calls a package's procedures at its mount, and `ObjectClient` keeps a scope's objects in a local database, follows queries live, and undoes changes.

```ts
import { createClient, ObjectClient } from "@destack/object/client";

const forge = createClient(forgePackage, { url: PackageMount.url(origin, forgePackage.id) });
await forge.release.list({ scope });
```

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
