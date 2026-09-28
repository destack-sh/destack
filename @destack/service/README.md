# @destack/service

Define, host and call Destack HTTP services, their triggers and their background work.

## Services

`defineService` declares procedures with their route, authentication, permission and audit.

```ts
import { defineProcedure, defineService } from "@destack/service";

export const notesService = defineService("notes", {
    list: defineProcedure({ authentication: "identity", permission: note.permission("read"), audit: false })
        .route({ method: "GET", path: "/notes" })
        .output(page(Note)),
    objects: { notebook, note },
});
```

## Servers

A `Server` authenticates each request, checks the procedure's permission on the call's target and runs its handler.

```ts
import { implement, Server, type ServiceContext } from "@destack/service/server";

const service = implement(notesService.router).$context<ServiceContext>();
await using server = Server.start({
    service: notesService,
    router: service.router({ list: service.list.handler(({ context }) => notebook.list(context)) }),
    access: { authorizer, database, target: async ({ input }) => note.reference(spaceId, input.id) },
    audience: packageId,
    scope: spaceId,
    resources,
    authenticate,
    authorizeHost,
    health: new Health("notes"),
    drainTimeout: 10_000,
});
```

## Workloads

A workload starts once per instance and returns the services and trigger handlers it implements.

```ts
import { defineWorkload, WorkloadInstance } from "@destack/service/workload";

export const workload = defineWorkload({
    name: "main",
    start: async (context) => {
        const notebook = new Notebook(database.get(context.resources));
        context.defer(() => notebook.close());

        return { services: [implementNotes(notebook)], triggers: [reminders.handle((occurrence) => notebook.remind(occurrence))] };
    },
});

const instance = await WorkloadInstance.start(workload, { resources, service: () => serverOptions });
const response = await instance.fetch(notesService, request);
```

## Clients

A service connection declares a dependency on a service, and a `ClientContext` binds it to the host's endpoint.

```ts
import { defineServiceConnection } from "@destack/service/declare";
import { ClientContext, safe } from "@destack/service/client";

export const notes = defineServiceConnection("notes", notesService);

const context = new ClientContext(configuration, { headers, bookmark });
context.bind(notes);
const result = await safe(notes.get(context.resources).update(input));
```

## Authentication

A `TokenIssuer` signs a verified caller, and a `TokenVerifier` checks the token at the receiving service.

```ts
import { TokenIssuer, TokenVerifier } from "@destack/service/authentication";

const issuer = new TokenIssuer({ authority: { kind: "global" }, issuer: accountOrigin, sign });
const { accessToken } = await issuer.issue(caller);

const verifier = new TokenVerifier({ authority: { kind: "global" }, issuer: accountOrigin, audience: packageId, keys });
const verified = await verifier.authenticate(request);
```

## Journal

A `Journal` runs each request once in one transaction and replays its outcome to retries.

```ts
import { defineJournal, Journal } from "@destack/service/database";
import { RequestFingerprint, RequestId } from "@destack/service/request";

export const journal = new Journal(defineJournal("journal"));

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
const client = createClient(notesService.router, { url, bookmark });
```

## Triggers

A package declares triggers, and the host delivers each event to its handler once per cause.

```ts
export const reminders = defineSchedule({ name: "reminders", timing: "cron", cron: "0 9 * * *", timezone: "Europe/Zurich", concurrency: "forbid", deadline: 60_000 });
export const pushes = defineWebhook({ name: "github", verification: "github", secret: webhookSecret });
export const published = defineWatch({ name: "published", object: note, where: Condition.eq("status", "published"), on: ["create", "update"], from: "snapshot" });

await instance.deliver(pushes, await WEBHOOK_SIGNATURES.github.verify(request, secret, Date.now()), signal);
```

## Trigger events

Each trigger kind delivers one event type.

| Trigger | Entry point | Event |
|---|---|---|
| `defineSchedule` | `@destack/service/schedule` | `ScheduleOccurrence` |
| `defineWebhook` | `@destack/service/webhook` | `WebhookDelivery` |
| `defineWatch` | `@destack/service/watch` | `ObjectChange` |

## Controllers

A `ControlLoop` runs level-triggered `Controller`s, which reconcile keys named by a database's committed changes.

```ts
import { ControlLoop, type Controller } from "@destack/service/control";

const expiry: Controller = {
    name: "expiry",
    watches: [session],
    keys: () => ["expiry"],
    list: async () => ["expiry"],
    reconcile: async () => ((await removeExpired()) ? 0 : undefined),
};
await new ControlLoop(database, [expiry], { report, lease: { holder: instanceId } }).run(signal);
```

## Outbox

An `Outbox` commits messages with their transaction and delivers them in order to a `Destination`.

```ts
import { Outbox, type Destination } from "@destack/service/outbox";

const inbox: Destination<Delivery> = { name: "inbox", message: Delivery, batch: 100, accept: (deliveries, { signal }) => home.accept(deliveries, { signal }) };
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

const operations = new OperationStore(defineOperation(Published, Progress), { concurrency: 4, capacity: 100, retention: 3_600_000, timeout: 60_000 });
const router = implementOperation(operations);
operations.start(caller.id, { completed: 0 }, async ({ signal, report }) => {
    const output = await publish({ signal });
    report({ completed: 1 });

    return { url: output.url };
});
```
