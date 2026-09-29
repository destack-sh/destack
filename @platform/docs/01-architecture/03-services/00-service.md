---
title: Service
description: APIs, connections, bindings and workloads.
---

# Service

- `@destack/service` defines HTTP services and clients with oRPC and Zod
- procedures declare input, output, errors, authentication, permission and audit
- calls carry identity, authorisation, cancellation and tracing

```text
Service                a named API
├─ Procedure           input, output, errors, access, audit
└─ Operation           long-running work with progress
Connection             a consumed service, bound by the host
Workload               services hosted together
└─ start(context)
```

- OpenAPI-compatible HTTP endpoints; SSE for streams; HTTP for files
- client entrypoints never load server code or credentials
- secret values and credentials never reach logs or traces

```ts
import { defineService, defineProcedure } from "@destack/service";
import { schema } from "@destack/schema";

export const notesService = defineService("notes", {
    list: defineProcedure({ authentication: "identity", permission: null, audit: false })
        .route({ method: "GET", path: "/notes" })
        .output(schema.array(schema.object({ id: schema.string(), title: schema.string() }))),
});
```

## Conventions

- creates accept a request ID for retries
- updates require the inspected revision ([AIP-154](https://google.aip.dev/154))

- long-running work returns an operation with durable progress ([AIP-151](https://google.aip.dev/151))
- pagination follows [AIP-158](https://google.aip.dev/158)
- mutations return persisted desired state; controllers report observed progress

## Connections

- `defineService` declares a provided API; `defineServiceConnection` declares a consumed one
- same-installation targets come from the build; cross-installation targets from space administration
- a binding selects a target and grants no permission

```ts
// connection/notes.ts
import { defineServiceConnection } from "@destack/service/declare";
import { notesService } from "@florian/notes/service";

export const notes = defineServiceConnection("notes", notesService);

// anywhere with a bound context
const result = await notes.get(context.resources).list();
```

- browsers call same-origin endpoints only and reject redirects; browser identity grants no server authority

## Workloads

- a workload exports `start(context)` and returns the services it hosts
- the context supplies identity, resources and bindings

```ts
// workload/workload.ts
export async function start(context: WorkloadContext): Promise<WorkloadImplementation> {
    const notebook = new Notebook(database.get(context.resources));

    return { services: { notes: implementService(notebook) } };
}
```

```ts
interface WorkloadContext {
    readonly resources: ResourceContext;
    readonly signal: AbortSignal;
    shutdown(): void;
    defer(dispose: () => void | PromiseLike<void>): void;
}
```

- shutdown cancels background work and drains services before releasing resources
- durable work belongs in jobs and outboxes, not in shutdown callbacks

- cloud and self-hosted deployments use the same APIs and rules
- bootstrap credentials come from host provisioning; vault root keys never depend on the vault
