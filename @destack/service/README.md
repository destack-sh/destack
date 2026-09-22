Define Destack HTTP services.

## Usage

```ts
import { schema } from "@destack/schema";
import { defineService, defineProcedure } from "@destack/service";
import { inspectService } from "@destack/service/inspect";

const notes = {
    list: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/notes" })
        .output(schema.array(schema.object({ id: schema.string(), title: schema.string() }))),
};

export const service = defineService({
    name: "notes",
    version: 1,
    protocol: "http",
    handler: "fetch",
}, notes);

const description = await inspectService(service, {
    info: { title: "Notes", version: "2026.9.0" },
});
```

Handle declared errors through the typed client.

```ts
import { safe } from "@destack/service/client";

const result = await safe(client.update(input));
if (result.isDefined && result.error.code === "CONFLICT") {
    console.log(result.error.data.revision);
} else if (result.error) {
    throw result.error;
}
```

Unexpected handler failures are logged through `@destack/telemetry` and returned as internal errors.
Clients and handlers automatically record RPC spans and duration metrics through stream completion.
The host starts telemetry providers and configures export; request inputs are excluded from metrics.

## Operations

Operations retain typed progress and results in memory until expiry or shutdown.

```ts
import { schema } from "@destack/schema";
import { defineOperation, defineOperationProcedures } from "@destack/service/operation";
import { implementOperation, OperationStore } from "@destack/service/server";

const result = schema.object({ url: schema.httpUrl() });
const progress = schema.object({ completed: schema.number().int().nonnegative() });

const operationDefinition = defineOperation(result, progress);
const operations = new OperationStore(operationDefinition, {
    concurrency: 4,
    capacity: 100,
    retention: 3600000,
    timeout: 60000,
});
const definition = defineOperationProcedures(operationDefinition);
const router = implementOperation(operations);

const operation = operations.start(caller.id, { completed: 0 }, async ({ signal, report }) => {
    const output = await publish({ signal });
    report({ completed: 1 });
    return { url: output.url };
});
```

## Hosting

The host supplies authentication and a Fetch listener; shutdown waits for response streams.

```ts
import { implementHealth, Server, ServiceHandler } from "@destack/service/server";
import { Health } from "@destack/service/health";

const health = new Health("publish");
await using server = await Server.start({
    handler: new ServiceHandler({ ...router, health: implementHealth(health) }, {
        health,
        authorize: ({ context, access, input }) => authorize(context.owner, access, input),
        audit: (event) => recordAudit(event),
    }),
    context: async (request) => ({ owner: (await authenticate(request)).id }),
    drainTimeout: 10000,
    dispose: () => operations.close(),
});

// pass requests from the host's listener
const response = await server.fetch(request);
server.health.set("not-serving");
server.health.set("serving");
```

`ServiceHandler` serves `/livez` and `/readyz`; protected and audited procedures require host callbacks.

```ts
import { health } from "@destack/service/health";
import { createClient } from "@destack/service/client";

const client = createClient({ health }, { url, headers });
const status = await client.health.check();
```

## Inspection

```ts
import { implementInspection } from "@destack/service/server";

const administration = {
    inspection: await implementInspection(service, {
        info: { title: "Publish", version: "1.0.0" },
    }),
};
```

## Workloads

Declare the HTTP handler referenced by a workload.

```ts
import { defineService } from "@destack/service";

export const web = defineService({ name: "web", version: 1, protocol: "http", handler: "fetch" });

export function fetch(request: Request): Response {
    return Response.json({ path: new URL(request.url).pathname });
}
```

## Schedules

```ts
import { defineSchedule } from "@destack/service/schedule";

export const reminders = defineSchedule({
    name: "reminders",
    version: 1,
    handler: "remind",
    timing: "cron",
    cron: "0 9 * * *",
    timezone: "Europe/Zurich",
    concurrency: "forbid",
    deadline: 60000,
});
```
