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

`Monitor` holds open segments in memory and writes full or old ones to the bucket as Parquet files, and the optional `channel` argument shares open entries between instances.

```ts
import { AuditRecorder } from "@destack/audit";
import { Monitor } from "@destack/monitor";
import { monitorService } from "@destack/monitor/service";
import { implementService } from "@destack/monitor/server";

const monitor = new Monitor(database, bucket, report, database.channel("monitor"));
void monitor.run(signal);

const record = AuditRecorder.service(journal, {
    package: monitorService.package,
    service: "monitor",
});
const service = implementService(monitor, { access, record });
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
