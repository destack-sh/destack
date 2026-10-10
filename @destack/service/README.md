# @destack/service

`defineProcedure` is oRPC's contract builder `oc` with a permission and an audit per procedure, `/server` is oRPC's `implement` served as OpenAPI, `ServiceError` is `ORPCError`, `defineTrigger` is a Kubernetes CronJob, a Standard Webhooks receiver and a change trigger, `/outbox` is the transactional outbox, `/authentication` is RFC 8693 token exchange, and `Loan` is the AT Protocol's inter-service token for one method (`lxm`).

```ts
// oc
const list = defineProcedure({
    authentication: "identity",
    permission: page.permission("read"),
    audit: false,
});
list.route({ method: "GET", path: "/pages" }).output(PageList); // oc.route().output()
const service = implement(pagesService.router).$context<ServiceContext>(); // oRPC's implement
throw new ServiceError("NOT_FOUND", { message: `no page ${id}` }); // ORPCError
await ServiceError.read(response, "MISDIRECTED_REQUEST"); // the 421's declared data: { scope, machine } or { scope, partition }
// CronJob's concurrencyPolicy and startingDeadlineSeconds
on: { schedule: { timing, concurrency: "forbid", deadline: 60_000, call } };
await new StandardSignature().sign(message, secret); // Standard Webhooks' webhook-signature headers
await outbox.append(inbox, deliveryId, delivery, transaction); // transactional outbox
await spaceIssuer.exchange(caller, { actor: installation, audience }); // RFC 8693 token exchange
await Loan.sign(sign, { home, person, space, method: Loan.method(page, "update") }); // AT Protocol lxm
```

## Services

`defineService` declares a service's procedures and object types, and `since` and `convert` keep calls of earlier releases working.

```ts
export const pagesService = defineService("pages", {
    objects: { page },
    search: defineProcedure({
        authentication: "identity",
        permission: page.permission("read"),
        audit: false,
    })
        .input(schema.object({ query: schema.string() }))
        .output(PageList),
    since: "2026.8.0",
});
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
    name: "github",
    on: { webhook: { signature: new GitHubSignature(), route: "/{repository}", secret } },
    call: (delivery) =>
        repository.call("push", { id: delivery.parameters.repository!, payload: delivery.payload }),
});
```

## Workloads

`defineWorkload` declares a `start` that runs once per instance and returns the served services, the package's triggers, and what it does with the WebSockets its host keeps open, as Durable Objects hibernate theirs.

```ts
export const workload = defineWorkload({
    name: "main",
    permissions: { space: [message.permission("send")] },
    start: async (context) => ({ services: [implementPages(context)], triggers: [pushes] }),
});

// a room's partition: accept a socket in a route, answer its messages through the host
start: async ({ installation, hibernation }) => ({
    services: [{ service: rooms, router: {}, fetch: () => hibernation.accept({ id, attachment }) }],
    sockets: { message: async (socket, bytes) => socket.send(bytes), close: async () => {} },
});
installation.partition; // the one partition the instance serves, as Destack-Partition names it
installation.at(Egress.address(installationId, spaceId)); // another installation, through the egress
request.headers.get(FORWARDED_URL_HEADER); // a route's request at ROUTE_PATH, as a Workers route hands one on
```

## Bindings

`defineServiceBinding` declares a dependency on a service, and `defineHttps` on an external HTTPS API, both called through the host's egress.

```ts
export const pages = defineServiceBinding("pages", pagesService);
export const github = defineHttps("github", { url: "https://api.github.com" });

const result = await safe(pages.get(context.resources).update(input)); // oRPC's safe
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
await new ControlLoop(database, [expiry, outbox.controller(inbox)], { report }).run(signal);
```

## Requests

`RequestId.create` stamps a UUIDv7, `RequestId.derive` derives the same UUIDv8 on every repeat (RFC 9562), and `RateLimit` is a token bucket.

```ts
const requestId = await RequestId.derive(reading.measuredAt, `${source} ${eventId}`);
const { isAdmitted, state, wait } = new RateLimit({ rate: 1, burst: 20 }).admit(kept, now);
```
