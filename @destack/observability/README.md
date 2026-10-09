# @destack/observability

`@destack/observability` is an OTLP/JSON receiver over `@destack/event` with OpenTelemetry's logs, spans and metrics as event kinds, Sentry's issues grouped by error type and in-app frames with resolve, ignore and reopen, Sentry's alert rules, and meters that measure a space's use for `@destack/finance`.

```ts
await receiver.receive({ scope, installation, instance, build }, "logs", exported); // an OTLP/HTTP collector
await client.observability.trace({ scope, trace }); // a trace's spans and logs
await client.observability.percentiles({ scope, where: 'name = "http.server.duration"', fold: "p99", group, step: 60_000, from, before });
await client.issue.update({ spaceId, requestId, id, until: { kind: "revision", after: issue.lastRevisionId } }); // Sentry's resolve in next release
await client.alertRule.create({ spaceId, requestId, name: "Error rate", condition, actions: [{ kind: "notify" }] }); // a Sentry metric alert
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

const observability = implementObservability({ events, spaces: { openBuild, measure, skus: UNIT_SKUS }, report });
```

## Alert rules

`defineAlertRule` declares an alert rule that each installation of the package keeps in its space, and a `call` action runs a method of the installation.

```ts
import { defineAlertRule } from "@destack/observability/declare";

export const rollback = defineAlertRule({
    name: "Roll back new fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
});
```

## Meters

`databaseStorage`, `bucketStorage`, `compute`, `egress` and `operations` measure a space's use in the units of the SKUs that rate them.

```ts
import { bucketStorage, compute, databaseStorage, egress, operations } from "@destack/observability/meter";
```

## Settings

`telemetryRetention`, `analyticsRetention` and `traceSampling` set how many days a space keeps its telemetry and analytics and the share of traces it keeps.

```ts
import { telemetryRetention, traceSampling } from "@destack/observability/setting";

export const personal = defineSpace({
    settings: {
        retention: { setting: telemetryRetention, value: 90, mode: "set" },
        sampling: { setting: traceSampling, value: 0.1, mode: "set" },
    },
});
```
