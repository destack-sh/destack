# @destack/monitor

Store, search and alert on the telemetry of Destack scopes.

## Ingestion

The monitor accepts OTLP/HTTP JSON from installations, one path per signal.

```ts
const exporter = OtlpExporter.http(
    `${cell}/@destack/monitor`,
    () => `Bearer ${installationToken}`,
    report,
);
// POST /v1/logs, /v1/traces and /v1/metrics
```

## Storage

`Monitor.open` writes entries to the bucket as Parquet files and lists them in `monitorDatabase`.

```ts
const catalog = await sqlite.connect(file, monitorDatabase);
await using monitor = await Monitor.open(catalog, bucket, report, catalog.channel("monitor"));
```

## Service

`implementMonitor` serves a monitor as a kind service of a cell.

```ts
const service = implementMonitor({
    monitor,
    callKey,
    cell: hostId,
    scopes: [space],
    uplink: spaces,
    openBuild,
    report,
});
```

## Search

`search` and `tail` read the entries of an installation.

```ts
const page = await client.monitor.search({
    scope,
    installation,
    severity: 17,
    from,
    before,
    limit: 100,
});
for await (const entry of await client.monitor.tail({ scope, installation })) {
    render(entry);
}
```

## Series

`series` aggregates a metric per time step.

```ts
const crashes = await client.monitor.series({
    scope,
    installation,
    name: "session.count",
    filter: "session.status = crashed",
    group: ["destack.build.manifest"],
    from,
    before,
    step: 3_600_000_000,
});
```

## Issues

The monitor groups each unexpected exception into an `issue` by its error type and in-app frames.

```ts
const { items } = await client.issue.list({
    spaceId,
    where: { status: "unresolved" },
    orderBy: { lastSeenAt: "desc" },
});
// issue.culprit:     "package-…/src/note.ts#Note.rename"
// issue.declaration: "package-…/src/note.ts#Note.rename:method"
```

## Issue events

The monitor writes the issue of each exception to `destack.issue`.

```ts
const events = await client.monitor.search({
    scope,
    installation,
    attributes: { "destack.issue": id },
    from,
    before,
    limit: 50,
});
```

## Triage

`issue.update` sets `until`, the condition that reopens a resolved or ignored issue.

```ts
const until = { kind: "revision", after: issue.lastRevisionId } as const;
await client.issue.update({ spaceId, requestId, id, until });
await client.issue.resolve({ spaceId, requestId, id });
await client.issue.update({ spaceId, requestId, id, until: { kind: "count", count: issue.count + 100 } });
await client.issue.ignore({ spaceId, requestId, id });
```

## Alert rules

An `alertRule` fires an `alert` when an issue opens or a series crosses a threshold.

```ts
await client.alertRule.create({
    spaceId,
    requestId,
    name: "Fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }],
});
await client.alertRule.create({
    spaceId,
    requestId,
    name: "Crashing sessions",
    condition: {
        kind: "series",
        name: "session.count",
        filter: "session.status = crashed",
        per: "session.status = *",
        aggregate: "value",
        window: { hours: 1 },
        comparison: "above",
        threshold: 0.02,
    },
    actions: [{ kind: "notify" }],
});
```

### Actions

A `call` action runs a method of the alert's installation, such as `rollBack`.

```ts
await client.alertRule.create({
    spaceId,
    requestId,
    name: "Fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
});
```

### Declared rules

`defineAlertRule` declares an alert rule each installation of the package keeps in its space.

```ts
import { defineAlertRule } from "@destack/monitor/declare";

export const rollback = defineAlertRule({
    name: "Roll back new fatal issues",
    condition: { kind: "issue", on: "open", filter: "level = fatal" },
    actions: [{ kind: "notify" }, { kind: "call", method: "rollBack" }],
});
```

## Masking

`search`, `tail` and `trace` mask values under `sensitive.` keys unless the caller may `unmask`.

```ts
log.info("user.invited", { "sensitive.email": invite.email, role: "editor" });
```

## Settings

`telemetryRetention` sets how many days entries stay searchable.

```ts
export const personal = defineSpace({
    settings: {
        retention: { setting: telemetryRetention, value: 90, mode: "set" },
        sampling: { setting: traceSampling, value: 0.1, mode: "set" },
    },
});
```
