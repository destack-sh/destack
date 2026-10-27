# @destack/observability

`@destack/observability` keeps a space's logs, spans, metric points, visits and actions, groups failures into issues, fires alerts, and declares the meters a space's use is priced by.

```ts
await client.span.list({ spaceId, where: { trace } });
await client.issue.resolve({ spaceId, requestId, id, until: { kind: "revision", after } }); // reopen on a later revision
await client.issue.ignore({ spaceId, requestId, id, until: { kind: "never" } });
```

## Events

`log`, `span`, `metric`, `session`, `visit` and `action` are events types, all but `metric` keyed by session, their personal attributes masked unless the reader may unmask them.

```ts
import { log, session } from "@destack/observability";

await client.log.list({ spaceId, where: { session: "session-1" } }); // one session's timeline
await events.series(session.eventKind(), { scopes }, { fold: "count", group: ["build", "status"] }); // release health
```

## OTLP

The `otlp` adapter takes OTLP/HTTP JSON exports at `/v1/traces`, `/v1/logs` and `/v1/metrics` into the request's scope, appending an `exception` event for each failure a log record or span carries.

```ts
import { OtlpExport, OtlpIntake } from "@destack/observability";

const stamped = new OtlpExport("logs", body).stamp({ installation, build }); // as the forwarding machine
await new OtlpIntake(store).append(caller, scope, "logs", stamped.body); // appends only, its retries once each
```

## Exceptions

Each `exception` event names its issue by installation and fingerprint, and the `issues` reader counts a minute of a space's exceptions into their issues at a time, as Sentry buffers its counters.

```ts
await client.exception.list({ spaceId, where: { issue } }); // an issue's exceptions, kept as long as its logs
```

## Issues and alerts

Failures group into issues that are assigned, resolved, ignored and regress; alert rules raise alerts over issues or folds of events, pending until they fire, announced and answered by their rule's actions unless a silence covers them.

```ts
await client.alertRule.create({
    spaceId,
    requestId,
    title: "Error rate",
    condition: {
        kind: "events",
        event: log.typeReference,
        where: "severity >= 17",
        fold: "count",
        group: ["installationId"],
        per: {},
        window: { minutes: 5 },
        comparison: "above",
        threshold: 0.05,
    },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }], // the call runs as the author, waiting on approval
    pending: { minutes: 2 },
    noData: "ok",
});
await client.silence.create({
    spaceId,
    requestId,
    matchers: [{ label: "installationId", operator: "=", value }],
    startsAt,
    endsAt,
    comment,
});
```

## Server

The package's server, its default export of `./server`, serves issues, alert rules, alerts, the responses to fired alerts and the `otlp` adapter.

```ts
import observabilityServer from "@destack/observability/server";

const store = new ObjectStore({ servers: [observabilityServer, notificationServer], ...options });
```

## Readings

A `reading` records one increment of a meter a space used, with the SKU and payer it is billed at, and its `metering` reader hands each hour's readings to the machine billing them, as `@destack/finance` does.

```ts
await store.mutate(machine, { id, calls: [reading.call("append", { ...reading.in(spaceId), items })] });
controllers: [reading.consume("metering", async (page) => bill(page.events))],
```

## Meters

`defineMeter` declares what a meter's events count and how a period's events fold into usage; the platform's meters measure a space's use in generic units that each vendor's pricing maps to its own SKUs.

```ts
const requests = defineMeter({
    name: "api.requests",
    description,
    aggregation: "sum",
    unit: "request",
});
import { cpu, machineHours, rowsRead } from "@destack/observability/meter";
```

## Settings

`telemetryRetention`, `analyticsRetention` and `traceSampling` set how many days a space keeps its telemetry and analytics, and the share of traces it keeps.

```ts
import { telemetryRetention, traceSampling } from "@destack/observability/setting";

export const personal = defineStack({
    settings: {
        retention: { setting: telemetryRetention, value: 90, mode: "set" },
        sampling: { setting: traceSampling, value: 0.1, mode: "set" },
    },
});
```
