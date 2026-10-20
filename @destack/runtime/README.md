# @destack/runtime

`@destack/runtime` declares a package's procedures and workloads, serves them over oRPC, and issues the tokens its calls carry.

```ts
const list = defineProcedure({
    package: Package.of(pages), // the package whose releases its callers speak
    authentication: "identity",
    permission: page.permission("read"),
    audit: false,
});
list.route({ method: "GET", path: "/pages" }).output(PageList);
const implementation = implement(pages.router).$context<RequestContext>();
throw new CallError("NOT_FOUND", { message: `no page ${id}` });
await CallError.read(response, "MISDIRECTED_REQUEST");
await issuer.issue({ caller, credential }, now); // a caller's verified authentication
client.notes.list({}, { context: { exchange: Exchange.of(served) } });
```

## Prior art

`defineProcedure` is oRPC's `oc`, `CallError` is `ORPCError`, `ControlLoop` leases each key as Kubernetes leader election does, and a token carries an RFC 8693 actor chain and an RFC 9396 grant.

```ts
throw new CallError("TOO_MANY_REQUESTS", { message, data: { retryAfter } }); // Retry-After
new ControlLoop(database, controllers, { clock, report, lease: { holder: instance } });
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
            requires: [{ packageId: pricing, name: "push" }], // the account's plan must include it
        },
    },
    since: "2026.8.0",
});

const forge = createClient(forgePackage, { url: PackageMount.url(origin, forgePackage.id) });
await forge.release.list({ scope });
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

## Controllers

`ControlLoop` converges keys in `reconcile` mode and follows them in `follow` mode, listing again once a change touches what the list read through its view.

```ts
const expiry: Controller = {
    name: "expiry",
    watches: [session],
    keys: () => ["expiry"],
    list: async () => ["expiry"],
    reconcile: async () => ((await removeExpired()) ? 0 : undefined),
};
const live: Controller = {
    name: "live",
    mode: "follow",
    watches: [session],
    list: async (view) => (await view.read(session).findMany()).map((row) => row.id), // reads recorded
    reconcile: (id, { signal }) => serve(id, signal),
};
await new ControlLoop(database, [expiry, live], { clock, report }).run(signal);
```

## Requests

`RequestId.create` stamps a UUIDv7, and `RequestId.derive` derives the same UUIDv8 on every repeat (RFC 9562).

```ts
const requestId = await RequestId.derive(reading.measuredAt, `${source} ${eventId}`);
```
