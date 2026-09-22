Define Destack HTTP services.

## Usage

```ts
import { schema } from "@destack/schema";
import { defineService, defineProcedure } from "@destack/service";
import { inspectService } from "@destack/service/inspect";

export const notesService = {
    list: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/notes" })
        .output(schema.array(schema.object({ id: schema.string(), title: schema.string() }))),
};

export const service = defineService(
    {
        name: "notes",
        version: 1,
        protocol: "http",
        handler: "fetch",
    },
    notesService,
);

const description = await inspectService(service, {
    info: { title: "Notes", version: "2026.9.0" },
});
```

```ts
import { safe } from "@destack/service/client";

const result = await safe(client.update(input));
if (result.isDefined && result.error.code === "CONFLICT") {
    console.log(result.error.data.revision);
} else if (result.error) {
    throw result.error;
}
```

## Operations

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

```ts
// server/server.ts
import { implement, type ServiceContext, type ServiceImplementation } from "@destack/service/server";
import { notesService } from "../service/index.ts";
import type { Notebook } from "../notebook/index.ts";

export function implementService(notebook: Notebook): ServiceImplementation {
    const service = implement(notesService).$context<ServiceContext>();

    return {
        router: service.router({
            list: service.list.handler(({ context }) => notebook.list(context.access)),
        }),
        authorize: async ({ context }) => { context.requireCaller(); },
    };
}

// server/index.ts
export * from "./server.ts";
```

```ts
import { Server } from "@destack/service/server";
import { implementService } from "./server/index.ts";
import { Health } from "@destack/service/health";

const health = new Health("notes");
await using server = await Server.start({
    ...implementService(notebook),
    health,
    audience: servicePackageId,
    spaceId,
    resources,
    authenticate,
    authorizeHost: authorizeInstallation,
    drainTimeout: 10000,
    dispose: () => database.close(),
});

// pass requests from the host's listener
const response = await server.fetch(request);
server.health.set("not-serving");
server.health.set("serving");
```

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

## Authentication

```ts
import { Server, ServiceContext, implement } from "@destack/service/server";

// application handlers receive the same context on every hosting target
const implementation = implement(notesService).$context<ServiceContext>();
const router = implementation.router({
    list: implementation.list.handler(({ context }) =>
        database
            .select()
            .from(note)
            .where(noteAccess.where(noteRead, spaceId, context.access)),
    ),
});
const server = await Server.start({
    router,
    audience: servicePackageId,
    spaceId,
    resources,
    authenticate,
    authorizeHost: authorizeInstallation,
    health,
    drainTimeout: 10000,
    authorize: authorizeApplication,
    audit: recordAudit,
});

// attach the worker or local Fetch listener
const response = await server.fetch(request);
```

```ts
import { TokenVerifier } from "@destack/service/authentication";

const verifier = new TokenVerifier({
    authority: { kind: "global" },
    issuer: accountOrigin,
    audience: servicePackageId,
    keys: new URL("/auth/jwks", accountOrigin),
});
const caller = await verifier.authenticate(request, spaceId);
const access = caller.context(servicePackageId, Date.now(), spaceId);

// authorize each operation against current records and credential restrictions
await authorize(database, access, permission, object);
```

```ts
import { TokenIssuer } from "@destack/service/authentication";

// configure signing only in the authority, with a host-verified Caller
const issuer = new TokenIssuer({
    authority: { kind: "global" },
    issuer: accountOrigin,
    sign: async (payload) => {
        const signed = await authentication.api.signJWT({ body: { payload } });
        return signed.token;
    },
});
const issued = await issuer.issue(caller);

// constrain a space's signing keys independently at the receiving service
const workloads = new TokenVerifier({
    authority: { kind: "space", spaceId },
    issuer: spaceIssuer,
    audience: servicePackageId,
    keys: spacePublicKeys,
});
```

## Requests

```ts
import { createRequestId } from "@destack/service/request";
import { defineRequestTable, IdempotencyStore } from "@destack/service/database";

export const accountRequest = defineRequestTable("account_request");
const requests = new IdempotencyStore(accountRequest);

// retain the same mutation key through retries within seven days
const requestId = createRequestId();
await database.transaction(async (transaction) => {
    await authorize(transaction, caller, permission);
    const request = { caller: caller.id, scope: accountId, procedure: "account.update", requestId };
    const claim = await requests.begin(
        transaction,
        request,
        { digest },
        (stored) => stored === digest,
    );
    if (claim.kind === "replay") {
        return claim.value;
    }

    const result = await updateAccount(transaction, input);
    await recordAudit(transaction, result);
    await requests.complete(transaction, request, result);

    return result;
});

// invoke from authorized host maintenance after the immutable retry deadlines
await requests.prune(database, 100);
```
