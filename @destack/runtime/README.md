# @destack/runtime

`@destack/runtime` declares a package's procedures, triggers and workloads, serves them over oRPC, and issues the tokens its calls carry.

```ts
const list = defineProcedure({
    authentication: "identity",
    permission: page.permission("read"),
    audit: false,
});
list.route({ method: "GET", path: "/pages" }).output(PageList);
const implementation = implement(pages.router).$context<RequestContext>();
throw new CallError("NOT_FOUND", { message: `no page ${id}` });
await CallError.read(response, "MISDIRECTED_REQUEST");
await outbox.append(inbox, deliveryId, delivery, transaction);
await issuer.issue({ ...person, delegates: [installation], grant }, now);
client.notes.list({}, { context: { exchange: Exchange.of(served) } });
```

## Prior art

`defineProcedure` is oRPC's `oc`, `CallError` is `ORPCError`, a schedule trigger is a Kubernetes CronJob, `/outbox` is a transactional outbox, and a token carries an RFC 8693 actor chain and an RFC 9396 grant.

```ts
on: { schedule: { timing, concurrency: "forbid", deadline: 60_000, call } };
await new StandardSignature().sign(message, secret);
```

## Packages

`definePackage` declares a package's handle from its objects, views, adapters, resources and secrets, and `defineAdapter` answers a protocol others define below the package's mount.

```ts
export const npm = defineAdapter("npm", { path: "/npm" });
export default definePackage({
    objects: { repository, release },
    adapters: { npm },
    since: "2026.8.0",
});

const forge = createClient(forgePackage, { url: PackageMount.url(origin, forgePackage.id) });
await forge.release.list({ scope });
```

## Triggers

`defineTrigger` declares what fires runs, each one object method call the space's machine makes later as the installation.

```ts
export const reminders = defineTrigger({
    name: "reminders",
    on: {
        schedule: {
            timing: { timing: "cron", cron: "0 9 * * *", timezone: "Europe/Zurich" },
            concurrency: "forbid",
            deadline: 60_000,
            call: reminder.call("send", {}),
        },
    },
});
export const pushes = defineTrigger({
    name: "pushes",
    on: { webhook: { signature: new StandardSignature(), path: "/{repository}", secret } },
    call: (delivery) =>
        repository.call("push", { id: delivery.parameters.repository!, payload: delivery.payload }),
});
```

## Workloads

`defineWorkload` declares a `start` that runs once per instance and returns the served packages, the package's triggers, and what it does with the WebSockets its host keeps open, as Durable Objects hibernate theirs.

```ts
export const workload = defineWorkload({
    name: "main",
    permissions: { space: [message.permission("send")] },
    start: async (context) => ({ packages: [implementPages(context)], triggers: [pushes] }),
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
```

## Bindings

`defineInstallationBinding` declares a dependency on an installation of another package, and `defineHttps` on an external HTTPS API, both called through the host's egress.

```ts
export const pages = defineInstallationBinding("pages", pagesPackage, {
    calls: [pagesPackage.router.page.update],
});
export const github = defineHttps("github", { url: "https://api.github.com" });

const result = await safe(pages.get(context.resources).page.update(input)); // oRPC's safe
const response = await github.get(context.resources).fetch("/repos/acme/app/issues");
```

## Controllers

`ControlLoop` runs `Controller`s over each listed key, converging a key in `reconcile` mode and following it in `follow` mode.

```ts
const expiry: Controller = {
    name: "expiry",
    watches: [session],
    keys: () => ["expiry"],
    list: async () => ["expiry"],
    reconcile: async () => ((await removeExpired()) ? 0 : undefined),
};
await new ControlLoop(database, [expiry, outbox.controller(inbox)], { clock, report }).run(signal);
```

## Requests

`RequestId.create` stamps a UUIDv7, `RequestId.derive` derives the same UUIDv8 on every repeat (RFC 9562), and `RateLimit` is a token bucket.

```ts
const requestId = await RequestId.derive(reading.measuredAt, `${source} ${eventId}`);
const { isAdmitted, state, wait } = new RateLimit({ rate: 1, burst: 20 }).admit(kept, now);
```
