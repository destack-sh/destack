# @destack/monitor

Store, search and follow the logs, traces and metrics of Destack scopes.

## Ingestion

The monitor's mount accepts OTLP/HTTP JSON from installations by `POST` on one path per signal.

```text
POST /v1/logs
POST /v1/traces
POST /v1/metrics
```

## Storage

`Monitor` is a telemetry store: it holds open segments in memory, writes full or old ones to the bucket as Parquet files, and catalogs them in a `monitorDatabase` of its own.

```ts
import { AuditRecorder } from "@destack/audit";
import * as sqlite from "@destack/db/bun";
import { Monitor, monitorDatabase } from "@destack/monitor";
import { monitorService } from "@destack/monitor/service";
import { implementMonitor } from "@destack/monitor/server";

// keep the catalog as its own file
const catalog = await sqlite.connect(file, monitorDatabase);

// share open entries between instances over the channel
const monitor = new Monitor(catalog, bucket, report, catalog.channel("monitor"));
void monitor.run(signal);

// serve it with the owner's policies, retention settings and history
const record = AuditRecorder.service(journal, {
    package: monitorService.package,
    service: "monitor",
});
const service = implementMonitor({ monitor, access, settings: database, record });
```

## Service

`search` and `tail` read an installation's entries, or the entries of the scope itself without `installation`, and require a permission per signal, such as `read-logs` for `search`.

```ts
const page = await client.monitor.search({
    scope: spaceId,
    installation,
    severity: 17,
    from,
    before: Date.now() * 1000,
    limit: 100,
});
for await (const entry of await client.monitor.tail({ scope: spaceId, installation })) {
    render(entry);
}
```

## Masking

Reads show `****` for each attribute value under a `sensitive.` key unless the reader has the `unmask` permission, and each unmasked read records a `monitor.unmask` audit event.

```ts
log.info("user.invited", { "sensitive.email": invite.email, role: "editor" });
```

## Settings

`telemetryRetention` sets how many days a space's entries stay searchable, and `traceSampling` sets the share of traces kept in addition to every failed or slow one.

```ts
import { telemetryRetention, traceSampling } from "@destack/monitor";
import { defineSpace } from "@destack/space";

export const personal = defineSpace({
    settings: {
        retention: { setting: telemetryRetention, value: 90, mode: "set" },
        sampling: { setting: traceSampling, value: 0.1, mode: "set" },
    },
});
```
