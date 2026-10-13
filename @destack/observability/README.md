# @destack/observability

`@destack/observability` is an OTLP/JSON receiver over `@destack/event` with OpenTelemetry's logs, spans and metrics as event kinds, Sentry's issues grouped by error type and in-app frames with resolve, ignore and reopen, Sentry's alert rules, and meters that measure a space's use for `@destack/finance`.

```ts
// an OTLP/HTTP collector
await receiver.receive({ scope, installation, instance, build }, "logs", exported);
await client.observability.trace({ scope, trace }); // a trace's spans and logs
await client.observability.percentiles({
    scope,
    where: 'name = "http.server.duration"',
    fold: "p99",
    group,
    step: 60_000,
    from,
    before,
});
// Sentry's resolve in next release
await client.issue.update({
    spaceId,
    requestId,
    id,
    until: { kind: "revision", after: issue.lastRevisionId },
});
// a Sentry metric alert
await client.alertRule.create({
    spaceId,
    requestId,
    title: "Error rate",
    condition,
    actions: [{ kind: "notify" }],
});
```

## Events

`log`, `span`, `metric`, `visit` and `action` are the event kinds a space's `EventStore` keeps.

```ts
import { OBSERVABILITY_KINDS } from "@destack/observability";

const tables = eventTables([...OBSERVABILITY_KINDS, usage]);
```

## Service

`implementObservability` serves observability over an event store, with issues, alert rules and meters when it observes spaces.

```ts
import { implementObservability } from "@destack/observability/server";

const observability = implementObservability({
    events,
    spaces: { openBuild, measure, skus: UNIT_SKUS },
    report,
});
```

## Alert rules

`defineAlertRule` declares an alert rule that each installation of the package keeps in its space, keyed by its export and shown by its title, and a `call` action runs a method of the installation.

```ts
import { defineAlertRule } from "@destack/observability/declare";

export const rollback = defineAlertRule({
    title: "Roll back new fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
});
```

## Meters

`METERS` measure a space's use in generic units, the same on every machine kind, which each vendor's pricing model maps to its own SKUs, grouped by the resource people see: Compute, Database, Storage and Network.

```ts
import {
    memory, // compute.memory, GB-seconds held
    cpu, // compute.cpu, CPU milliseconds
    requests, // compute.requests
    databaseBytes, // database.bytes, averaged over the month
    rowsRead, // database.rows.read
    rowsWritten, // database.rows.written
    storageBytes, // storage.bytes, averaged over the month
    storageWrites, // storage.writes
    storageReads, // storage.reads
    egress, // network.egress, bytes
} from "@destack/observability/meter";
import { Reading } from "@destack/observability/server";

Reading.counted(rowsRead, [{ scope, installation, value: 1200 }]);
```

## Machine hours

`machineHours` measures the hours a dedicated machine stays awake, beside the spaces' meters.

```ts
import { machineHours } from "@destack/observability/meter";

machineHours.reference; // { packageId, name: "machine.hours" }
```

## Settings

`telemetryRetention`, `analyticsRetention` and `traceSampling` set how many days a space keeps its telemetry and analytics and the share of traces it keeps.

```ts
import { telemetryRetention, traceSampling } from "@destack/observability/setting";

export const personal = defineStack({
    settings: {
        retention: { setting: telemetryRetention, value: 90, mode: "set" },
        sampling: { setting: traceSampling, value: 0.1, mode: "set" },
    },
});
```
