# @destack/service

Define, host and call Destack HTTP services, their triggers and their background work.

## Services

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

## Releases

A server serves the releases from `since` up to its own, converting earlier inputs through each later release's `convert`.

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

## Servers

A `Server` authenticates each request, checks the procedure's permission on the call's target and runs its handler.

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

## Errors

A handler hides a refused object as a missing one with `conceal`, and the audit records the denial `denialOf` reads from its cause.

```ts
import { conceal, denialOf, ServiceError } from "@destack/service/error";

const denial = new ServiceError("FORBIDDEN", { message: "permission denied: read" });
throw isReader ? denial : conceal(denial, `no note ${id}`); // the caller sees NOT_FOUND
denialOf(failure)?.code; // "FORBIDDEN" for both
```

## Workloads

A workload starts once per instance and returns the services and webhooks it serves.

```ts
import { defineWorkload, WorkloadInstance } from "@destack/service/workload";

export const workload = defineWorkload({
    name: "main",
    start: async (context) => {
        const notebook = new Notebook(database.get(context.resources));
        context.defer(() => notebook.close());

        return { services: [implementNotes(notebook)], webhooks: [pushes] };
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

## Runners

A `WorkloadRunner` serves one workload as a host's start message asks, opening each resource binding through its declaration's connectors.

```ts
import { WorkloadRunner } from "@destack/service/workload";

// the runner holds no tokens: it calls services through the host's egress with the host's secret
const workload = await WorkloadRunner.start(
    { workload, resources, history, access },
    start,
    startTelemetry,
    report,
);
const response = await workload.fetch(request);
await workload.close();
```

On Bun, `runWorkload` reads the host's lines from standard input and serves the runner on a loopback port.

```ts
import { runWorkload } from "@destack/service/bun";

await runWorkload(runner, lines(process.stdin), async (ready) =>
    process.stdout.write(`${JSON.stringify(ready)}\n`),
);
```

The host and the runner exchange one JSON line per message.

| Line | Direction | Carries |
|---|---|---|
| `WorkloadStart` | host to runner, first | instance, scope, installation, resource bindings, the secret both sides prove requests with, the host's egress, trace sampling ratio |
| `WorkloadReady` | runner to host, first | the loopback port it serves on |

## Clients

A service binding declares a dependency on a service; a stack binds it to an address, and its client calls the service through the host's egress as the workload's installation.

```ts
import { defineServiceBinding } from "@destack/service";
import { safe } from "@destack/service/client";

export const notes = defineServiceBinding("notes", notesService);

const result = await safe(notes.get(context.resources).update(input));
```

`Egress` writes and reads the egress paths: `<egress>/<address>/<path>`, the address an installation (`notes`, `notes.work.acme`) or a package's service (`@destack/audit`).

```ts
import { Egress } from "@destack/service";

const url = Egress.url(start.egress, "@destack/audit");
const routed = Egress.route(request); // { address, request below it }
```

## Authentication

A `TokenIssuer` signs a verified caller, and a `TokenVerifier` checks the token at the receiving service.

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

## Journal

A `Journal` runs each request once in one transaction and replays its outcome to retries.

```ts
import { defineJournal, Journal } from "@destack/service/database";
import { RequestFingerprint, RequestId } from "@destack/service/request";

export const journal = new Journal(defineJournal("journal"));
export const accountJournal = defineJournal("journal", { tier: "global" });

const request = { caller: caller.id, scope: spaceId, requestId: RequestId.create() };
const fingerprint = await RequestFingerprint.hash(AccountUpdate, input);
const account = await journal.execute(database, request, fingerprint, {
    authorize: (transaction) => authorization.within(transaction).require(permission, target),
    run: (transaction) => updateAccount(transaction, input),
});
```

## Pages

A `Page` reads a cursor request and cuts the rows into a page with the next cursor.

```ts
import { Page } from "@destack/service/page";

const request = new Page(input, [spaceId], schema.string());
const rows = await readNotes({ after: request.after, limit: request.limit + 1 });
const result = request.result(rows, (row) => row.id);
```

## Bookmarks

A `Bookmark` carries the log watermarks a client has seen, so later reads include its own writes.

```ts
import { Bookmark } from "@destack/service/bookmark";

const bookmark = new Bookmark();
const client = createClient(notesService, { url, bookmark });
```

## Triggers

A trigger is a cause of runs, and every run is one object method call the space's cell makes later as the installation.

| Trigger | Entry point | Cause | Where it is followed |
|---|---|---|---|
| `defineSchedule` | `@destack/service/schedule` | a timing and a static call | the cell |
| `defineWebhook` | `@destack/service/webhook` | a signed request, verified with the installation's secret | the workload |
| `defineWatch` | `@destack/service/watch` | an admitted change of the installation's own objects | the workload's object server |

```ts
export const reminders = defineSchedule({
    name: "reminders",
    timing: "cron",
    cron: "0 9 * * *",
    timezone: "Europe/Zurich",
    concurrency: "forbid",
    deadline: 60_000,
    call: reminder.calls().send({}),
});
export const pushes = defineWebhook({
    name: "github",
    verification: "github",
    route: "/{repository}",
    secret: ({ repository }, resources) => vault.get(resources).read(repository),
    call: (delivery) =>
        repository.calls().push({ id: delivery.parameters.repository!, payload: delivery.payload }),
});
export const published = defineWatch({
    name: "published",
    object: note,
    where: Condition.eq("status", "published"),
    on: ["create", "update"],
    from: "snapshot",
    call: (change) => note.calls().index({ id: change.after!.id }),
});
```

A method sends a call to run once its transaction commits; it runs on the authority the calling person lent the installation.

```ts
await call.send({ call: mail.calls().welcome({ userId }) });
```

Outside a method, a workload sends through the cell recording its runs, with a request identifier when it may retry.

```ts
await context.runs.send({ call: digest.calls().send({}), at: Date.now() + 60_000, requestId });
```

## Controllers

A `ControlLoop` runs `Controller`s, which keep the state of each listed key matching a database's rows: `reconcile` mode converges a key and returns, `follow` mode keeps its process running until its list drops it.

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

## Outbox

An `Outbox` commits messages with their transaction and delivers them in order to a `Destination`.

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

## Observables

An `Observable` holds a current value and yields it again after each change.

```ts
import { Observable } from "@destack/service/observable";

const status = new Observable<Status>("starting");
status.set("ready");
for await (const value of status.watch(signal)) {
    render(value);
}
```

## Timers

`wait` pauses for a delay, and `RetryPolicy` spaces the attempts of failing work.

```ts
import { RetryPolicy, wait } from "@destack/service/timer";

await wait(1000, { signal });
await RetryPolicy.pause(RetryPolicy.of({ maximumInterval: 60_000 }), failures, signal);
```

## Operations

An `OperationStore` runs long work in memory, with progress, cancellation and a deadline.

```ts
import { implementOperation, OperationStore } from "@destack/service/server";

const operations = new OperationStore(defineOperation(Published, Progress), {
    concurrency: 4,
    capacity: 100,
    retention: 3_600_000,
    timeout: 60_000,
});
const router = implementOperation(operations);
operations.start(caller.id, { completed: 0 }, async ({ signal, report }) => {
    const output = await publish({ signal });
    report({ completed: 1 });

    return { url: output.url };
});
```
