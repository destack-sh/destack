Define, host and call Destack HTTP services.

## Services

A service declares procedures, each with its authentication, the permission it requires and whether it is audited.

```ts
import { schema } from "@destack/schema";
import { defineService, defineProcedure } from "@destack/service";

export const notesService = defineService("notes", {
    list: defineProcedure({ authentication: "identity", permission: note.permission("read"), audit: false })
        .route({ method: "GET", path: "/notes" })
        .output(schema.array(Note)),

    // object types route their procedures under their key
    objects: { notebook, note },
});
```

## Hosting

A `Server` authenticates each request, checks the declared permission on the call's target, and drains on close.

```ts
import { Server, implement, type ServiceContext } from "@destack/service/server";

const service = implement(notesService.router).$context<ServiceContext>();
await using server = Server.start({
    service: notesService,
    router: service.router({
        list: service.list.handler(({ context }) => notebook.list(context.authorization!)),
    }),
    access: { authorizer, database, target: async ({ input }) => note.reference(spaceId, input.id) },
    audience: servicePackageId,
    scope: spaceId,
    resources,
    authenticate,
    authorizeHost,
    audit,
    health: new Health("notes"),
    drainTimeout: 10000,
});
const response = await server.fetch(request);
```

## Workloads

A workload starts once per instance and returns the services and triggers it implements.

```ts
import { defineWorkload, WorkloadInstance } from "@destack/service/workload";

export const workload = defineWorkload({
    name: "main",
    compute: { limits: { memory: 512 } },
    start: async (context) => {
        const notebook = new Notebook(database.get(context.resources));
        context.defer(() => notebook.close());

        return { services: [implementService(notebook)], triggers: [reminders.handle((occurrence) => notebook.remind(occurrence))] };
    },
});

const instance = await WorkloadInstance.start(workload, { resources, service: serviceOptions });
const response = await instance.fetch(notesService, request);
```

## Triggers

A trigger is a declared event source, which the host delivers once per cause and records as a run.

| Trigger | Declared with | Event | Cause |
|---|---|---|---|
| schedule | `defineSchedule` | `ScheduleOccurrence { scheduledAt }` | the occurrence time |
| webhook | `defineWebhook` | `WebhookDelivery { id, event, payload, receivedAt }` | the delivery identifier |
| subscription | `defineSubscription` | `SubscriptionChange { position, key?, operation, before?, after? }` | the log position, and the row key of a snapshot's rows |

A package declares its triggers, and the host verifies and delivers each event.

```ts
export const reminders = defineSchedule({ name: "reminders", timing: "cron", cron: "0 9 * * *", timezone: "Europe/Zurich", concurrency: "forbid", deadline: 60000 });
export const pushes = defineWebhook({ name: "github", verification: "github", secret: webhookSecret });
export const published = defineSubscription({ name: "published", object: note, where: Condition.eq("status", "published"), on: ["create", "update", "delete"], from: "snapshot" });

await instance.deliver(pushes, await WEBHOOK_SIGNATURES.github.verify(request, secret, Date.now()), request.signal);
```

A webhook's verification names the scheme its sender signs with.

| Verification | Headers | Signed content |
|---|---|---|
| `standard` | `webhook-id`, `webhook-timestamp`, `webhook-signature: v1,<base64 HMAC-SHA256>` | `id.timestamp.body` with the `whsec_` base64 secret, within five minutes |
| `github` | `x-github-delivery`, `x-github-event`, `x-hub-signature-256: sha256=<hex HMAC-SHA256>` | the body with the secret's UTF-8 bytes |

A subscription consumes its object type's changes exactly.

| Rule | Reason |
|---|---|
| rows entering the condition are created, rows leaving it deleted | the condition is the subscribed set |
| each change is admitted by the access the installation had when it committed | the log replays history |
| `from: "snapshot"` delivers every admitted row at the start as created | consumers building a projection start complete |
| the log keeps changes for `maxLag` milliseconds, a week by default, then the subscription fails | a stuck consumer never pins the log forever |
| handlers apply a change through a Journal request derived from its position and key | a host may deliver a change again |

## Clients

A connection declares a dependency on a service, and the host binds it to an endpoint.

```ts
import { defineServiceConnection } from "@destack/service/declare";
import { ClientContext, safe } from "@destack/service/client";

export const notes = defineServiceConnection("notes", notesService);

const context = new ClientContext(configuration, { headers, bookmark });
context.bind(notes);
const result = await safe(notes.get(context.resources).update(input));
if (result.isDefined && result.error.code === "CONFLICT") {
    retry(result.error.data.revision);
}
```

## Authentication

A `TokenIssuer` signs a verified caller, and a `TokenVerifier` checks the token and the issuer's authority at the receiving service.

```ts
import { TokenIssuer, TokenVerifier } from "@destack/service/authentication";

const issuer = new TokenIssuer({ authority: { kind: "global" }, issuer: accountOrigin, sign });
const { accessToken } = await issuer.issue(caller);

const verifier = new TokenVerifier({
    authority: { kind: "space", spaceId },
    issuer: spaceIssuer,
    audience: servicePackageId,
    keys: new URL("/auth/jwks", spaceIssuer),
});
const verified = await verifier.authenticate(request, spaceId);
```

## Journal

A journal runs each request once in one transaction and replays its outcome, including final failures, to retries.

```ts
import { RequestId, RequestFingerprint } from "@destack/service/request";
import { defineJournal, Journal } from "@destack/service/database";

export const journal = new Journal(defineJournal("journal"));

const request = { caller: caller.id, scope: spaceId, requestId: RequestId.create() };
const digest = (await RequestFingerprint.hash(input)).toHex();
const account = await journal.execute(database, request, { digest }, {
    authorize: (transaction) => authorization.within(transaction).require(permission, target),
    run: (transaction) => updateAccount(transaction, input),
});

// host maintenance after the retry deadlines
await journal.prune(database, 100);
```

## Bookmarks

A response carries the watermarks its writes reached, and a client sends them back so its later reads see its own writes.

```ts
import { Bookmark } from "@destack/service/bookmark";

const bookmark = new Bookmark();
const client = createClient(notesService.router, { url, bookmark });

// in a handler
context.observed.observe(await server.watermark(scope));
await server.reach(context, scope);
```

## Operations

An operation store runs long work in memory, with progress, cancellation and a deadline.

```ts
import { defineOperation } from "@destack/service/operation";
import { implementOperation, OperationStore } from "@destack/service/server";

const operations = new OperationStore(defineOperation(Published, Progress), {
    concurrency: 4,
    capacity: 100,
    retention: 3600000,
    timeout: 60000,
});
const router = implementOperation(operations);

operations.start(caller.id, { completed: 0 }, async ({ signal, report }) => {
    const output = await publish({ signal });
    report({ completed: 1 });

    return { url: output.url };
});
```
