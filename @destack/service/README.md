# @destack/service

Define, host and call Destack HTTP services, their triggers and their background work.

## Declarations

`defineService` declares procedures with their route, authentication, permission and audit.

```ts
import { defineProcedure, defineService } from "@destack/service";

export const notesService = defineService("notes", {
    list: defineProcedure({
        authentication: "identity",
        permission: note.permission("read"),
        audit: false,
    })
        .route({ method: "GET", path: "/notes" })
        .output(page(Note)),
    objects: { notebook, note },
});
```

### Releases

`since` names the earliest release a service serves, and each later release's `convert` upgrades earlier inputs.

```ts
const search = defineProcedure({
    authentication: "identity",
    permission: null,
    audit: false,
    convert: {
        "2026.9.0": {
            query: Expression.column("text"), // renamed from text, which is then dropped
            limit: Expression.coalesce(Expression.column("limit"), Expression.literal(50)),
        },
    },
}).input(schema.object({ query: schema.string(), limit: schema.number().int() }));

export const searchService = defineService("search", { search, since: "2026.8.0" });
// every client sends the release it was built against in Destack-Version
```

### Triggers

`defineTrigger` declares what fires runs, each one object method call the space's cell makes later as the installation.

```ts
import { GitHubSignature } from "@destack/service/github";
import { defineTrigger } from "@destack/service/trigger";

export const reminders = defineTrigger({
    name: "reminders",
    on: {
        schedule: {
            timing: { timing: "cron", cron: "0 9 * * *", timezone: "Europe/Zurich" },
            concurrency: "forbid",
            deadline: 60_000,
            call: reminder.calls().send({}),
        },
    },
});
export const pushes = defineTrigger({
    name: "github",
    on: {
        webhook: {
            signature: new GitHubSignature(),
            route: "/{repository}",
            secret: ({ repository }: WebhookParameters, resources) =>
                vault.get(resources).read(repository!),
        },
    },
    call: (delivery) =>
        repository.calls().push({ id: delivery.parameters.repository!, payload: delivery.payload }),
});
export const published = defineTrigger({
    name: "published",
    on: {
        change: {
            object: note,
            where: { status: "published" },
            operations: ["create", "update"],
            from: "snapshot",
        },
    },
    call: (change) => note.calls().index({ id: change.after!.id }),
});
```

### Trigger kinds

`Trigger.kind` reads a trigger's kind from its `on` key.

```ts
Trigger.kind({ schedule }); // a timing, running the schedule's call, followed by the cell
Trigger.kind({ change }); // an admitted change of the installation's own objects, followed by its object server
Trigger.kind({ webhook }); // a signed delivery verified with the installation's secret, followed by the workload
```

### Webhook signatures

A webhook trigger names the `WebhookSignature` that verifies its deliveries, and the manifest lists the signature's `name` as its `verification`.
`StandardSignature` verifies Standard Webhooks, and `GitHubSignature` from `@destack/service/github` verifies GitHub's `x-hub-signature-256`.

```ts
const headers = await new StandardSignature().sign(message, secret);
const delivery = await new GitHubSignature().verify(request, secret, parameters, Date.now());
```

### Schedules

`Schedule` from `@destack/service/schedule` checks a schedule's timing and finds its occurrences on every server runtime.

```ts
Schedule.require(timing); // the timing, once its calendar, time zone and range are valid
Schedule.following(timing, after); // the next occurrence after a time, undefined once none follows
Schedule.recent(timing, after, until, limit); // { due, earlier }: the latest occurrences, and how many came before
```

## Service

`Server` authenticates each request, checks the procedure's permission on the call's target and runs its handler.

```ts
import { implement, Server, type ServiceContext } from "@destack/service/server";

const service = implement(notesService.router).$context<ServiceContext>();
await using server = Server.start({
    service: notesService,
    router: service.router({ list: service.list.handler(({ context }) => notebook.list(context)) }),
    access: {
        authorizer,
        database,
        target: async ({ input }) => note.reference(spaceId, input.id),
    },
    audience: packageId,
    scope: spaceId,
    resources,
    authenticate,
    authorizeHost,
    health: new Health("notes"),
    drainTimeout: 10_000,
});
```

### Authentication

`TokenIssuer` signs a verified caller, and `TokenVerifier` checks the token at the receiving service.

```ts
import { TokenIssuer, TokenVerifier } from "@destack/service/authentication";

const issuer = new TokenIssuer({ authority: { kind: "universe" }, issuer: accountOrigin, sign });
const { accessToken } = await issuer.issue(caller); // a caller without a space gets a token for universe services

const verifier = new TokenVerifier({
    authority: { kind: "universe" },
    issuer: accountOrigin,
    audience: packageId,
    keys,
});
const verified = await verifier.authenticate(request, spaceId); // any space when spaceId is absent
```

```ts
// a token names each installation with the deployment it runs in, or names its installation as subject alone under its own key
const workload = {
    subject: installation,
    deployments: [{ subject: installation, id: deploymentId }],
};
const external = { subject: installation, credential: { kind: INSTALLATION_KEY, id: keyId } };
```

```ts
// the server finds the principal's standing object, requires represent of the caller and decides as the principal with the caller as actor (RFC 8693)
import { Represented } from "@destack/service/authentication";

const fetch = Represented.fetch(hostFetch, principal.space.reference(Scope.universe.id, spaceId));
await connect({ url, fetch }).repository.open({ accountId, id, mode: "read" }); // decided as the space
```

### Granted calls

A procedure declared `granted` serves an installation only when its space granted it the call, which the installation's service binding declares as typed procedures and the host's egress carries in its token.

```ts
export const directory = {
    recipient: defineProcedure({
        authentication: "identity",
        permission: null,
        audit: "access",
        granted: true,
    }),
};
export const accounts = defineServiceBinding("account", accountService, {
    calls: [accountService.router.directory.recipient], // the egress grants "directory.recipient"
});
```

### Pages

`Page` reads a cursor request and cuts the rows into a page with the next cursor.

```ts
import { Page } from "@destack/service/page";

const request = new Page(input, [spaceId], schema.string());
const rows = await readNotes({ after: request.after, limit: request.limit + 1 });
const result = request.result(rows, (row) => row.id);
```

### Bookmarks

`Bookmark` holds the log watermarks a client has seen, and a request waits for them before it reads.

```ts
import { Bookmark } from "@destack/service/bookmark";

const bookmark = new Bookmark();
const client = createClient(notesService, { url, bookmark });
```

### Sent calls

`call.send` sends a call to run once its transaction commits, on the authority the caller lent the installation.

```ts
await call.send({ call: mail.calls().welcome({ userId }) });
```

### Run records

`RunEvent.requestId` derives the request an event records its run once under.

```ts
RunEvent.requestId({ at: 1_767_225_600_000 }); // "0001767225600000"
RunEvent.requestId({ delivery: { id: "72d3162e", path: "/acme" } }); // "/acme 72d3162e"
RunEvent.requestId({ change: { position, key: "note-1" } }); // "<epoch>/<sequence>/note-1", in log order
```

### Outbox

`Outbox` commits messages with their transaction and delivers them in order to a `Destination`.

```ts
import { Outbox, type Destination } from "@destack/service/outbox";

const inbox: Destination<Delivery> = {
    name: "inbox",
    message: Delivery,
    batch: 100,
    accept: (deliveries, { signal }) => home.accept(deliveries, { signal }),
};
const outbox = new Outbox(database);
await outbox.append(inbox, deliveryId, delivery, transaction);
await new ControlLoop(database, [outbox.controller(inbox)], { report }).run(signal);
```

### Controllers

`ControlLoop` runs `Controller`s over each listed key: `reconcile` mode converges a key and returns, and `follow` mode runs until the list drops the key.

```ts
import { ControlLoop, type Controller } from "@destack/service/control";

const expiry: Controller = {
    name: "expiry",
    watches: [session],
    keys: () => ["expiry"],
    list: async () => ["expiry"],
    reconcile: async () => ((await removeExpired()) ? 0 : undefined),
};
const stream: Controller = {
    name: "stream",
    mode: "follow",
    watches: [subscription],
    list: async () => subscriptions(),
    reconcile: async (key, { signal }) => (await follow(key, signal), undefined),
};
await new ControlLoop(database, [expiry, stream], { report, lease: { holder: instanceId } }).run(
    signal,
);
```

### Observables

`Observable` keeps a current value and yields it again after each change.

```ts
import { Observable } from "@destack/service/observable";

const status = new Observable<Status>("starting");
status.set("ready");
for await (const value of status.watch(signal)) {
    render(value);
}
```

### Timers

`wait` pauses for a delay, and `RetryPolicy` spaces the attempts of failing work.

```ts
import { RetryPolicy, wait } from "@destack/service/timer";

await wait(1000, { signal });
await RetryPolicy.pause(RetryPolicy.of({ maximumInterval: 60_000 }), failures, signal);
```

## Client

`defineServiceBinding` declares a dependency on a service, which a stack binds to an address, or which its space binds to the `address` it names, and its client calls through the host's egress.

```ts
import { defineServiceBinding } from "@destack/service";
import { safe } from "@destack/service/client";

export const notes = defineServiceBinding("notes", notesService);
export const mail = defineServiceBinding("mail", mailService, { address: "@destack/mail" }); // a platform service

const result = await safe(notes.get(context.resources).update(input));
```

### Egress

`Egress` writes and reads the egress paths: `<egress>/<address>/<path>`, the address an installation (`notes`, `notes.work.acme`) or a package's service (`@destack/audit`).

```ts
import { Egress } from "@destack/service";

const url = Egress.url(start.egress, "@destack/audit");
const routed = Egress.route(request); // { address, request below it }
```

## Workload

`defineWorkload` declares a `start` that runs once per instance and returns the served services and the package's triggers.

```ts
import { defineWorkload, WorkloadInstance } from "@destack/service/workload";

export const workload = defineWorkload({
    name: "main",
    start: async (context) => {
        const notebook = new Notebook(database.get(context.resources));
        context.defer(() => notebook.close());

        return { services: [implementNotes(notebook)], triggers: [pushes] };
    },
});

const instance = await WorkloadInstance.start(workload, {
    resources,
    history,
    access,
    service: () => serverOptions,
});
const response = await instance.fetch(notesService, request);
```

```ts
// placement lists where the universe's deployer may run a platform workload: in the universe, residency, space or host tier
export const forgeWorkload = defineWorkload({
    name: "forge",
    placement: ["residency"],
    start: (context) => ({ services: [forge(context)] }),
});
```

### Workload runs

`runs.send` sends a call from a workload outside a method, with a request identifier when it may retry.

```ts
await context.runs.send({ call: digest.calls().send({}), at: Date.now() + 60_000 }, { requestId });
```

### Runners

`WorkloadRunner` serves one workload of a build as a host's start message asks, opening each resource binding through its declaration's connectors and giving its installation the build's own files, such as its catalogs.

```ts
import { WorkloadRunner } from "@destack/service/workload";

// the runner has no tokens: it calls services through the host's egress with the host's secret
const workload = await WorkloadRunner.start(
    { workload, resources, history, access },
    start,
    build, // context.installation.build reads the package's own files, such as Catalog.read(build)
    startTelemetry,
    report,
);
const response = await workload.fetch(request);
await workload.close();
```

### Runner messages

`WorkloadStart` is the host's first line to a runner, and `WorkloadReady` the runner's answer once it serves.

```ts
const start: WorkloadStart = {
    instance, // the holder name of its leases
    scope: spaceId,
    installation,
    bindings, // the resources the installation binds, by resource name
    secret, // the secret both sides prove requests with
    egress: "http://127.0.0.1:8080/.destack/egress",
    sampling: 0.1, // the share of traces kept beside failed and slow ones
    callKey, // the installation's journal key as hexadecimal
};
const ready: WorkloadReady = { port: 41_234 }; // the loopback port serving the package's service
```

### Bun runners

`runWorkload` reads the host's lines from standard input on Bun and serves the runner on a loopback port.

```ts
import { runWorkload } from "@destack/service/bun";

await runWorkload(runner, lines(process.stdin), async (ready) =>
    process.stdout.write(`${JSON.stringify(ready)}\n`),
);
```

### Cloudflare runners

`DurableObjectWorkload` holds one workload in a Durable Object: it starts the workload before the object's first event, hands the object's storage to the workload as its controllers' alarm, and settles each alarm once no controller is due.

```ts
import { DurableObjectWorkload } from "@destack/service/cloudflare";

export class DurableObjectAccount extends DurableObjectWorkload {
    constructor(state: DurableObjectState, environment: Environment) {
        super(state, (alarm) => start(environment, alarm)); // anything with fetch(request) and alarm(deadline)
    }
}
```

## Errors

`conceal` hides a refused object as a missing one, and `denialOf` reads the denial the audit records from its cause.

```ts
import { conceal, denialOf, ServiceError } from "@destack/service/error";

const denial = new ServiceError("FORBIDDEN", { message: "permission denied: read" });
throw isReader ? denial : conceal(denial, `no note ${id}`); // the caller sees NOT_FOUND
denialOf(failure)?.code; // "FORBIDDEN" for both
```

```ts
// a code declares its status: UNAVAILABLE 503 and MOVED 421 beside oRPC's common codes
throw new ServiceError("UNAVAILABLE", { message: "the instance is starting" }); // 503
ServiceError.status("MOVED"); // 421
```

`ServiceError.of` reads the service error a failure is or names through its `toServiceError`, and `reportError` answers every other failure as `INTERNAL_SERVER_ERROR`.

```ts
ServiceError.of(new DatabaseError("DUPLICATE", "a record with the same unique key exists")); // CONFLICT 409
ServiceError.of(new Error("the disk is full")); // undefined
reportError(new Error("the disk is full")); // INTERNAL_SERVER_ERROR 500, "internal server error"
```

## Tests

`@destack/service/test` signs calls as test principals with `subjectContext` and `testCallKey`, and `testInstallation` gives served objects an installation with an empty build, an idle cell and a directory where nobody lives.

```ts
const server = new ObjectServer({
    objects,
    database,
    callKey: testCallKey,
    origin: { package: task.package, service: "tasks" },
    installation: await testInstallation(task.package, spaceId),
});
await server.call(task, "create", input, subjectContext(alice, spaceId));
```
