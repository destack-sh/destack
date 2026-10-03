Store, search and follow the logs, traces and metrics of Destack scopes.

## Ingestion

Installations and hosts send OTLP/HTTP JSON to these paths below the monitor's mount.

| Path          | Signal        |
| ------------- | ------------- |
| `/v1/logs`    | logs          |
| `/v1/traces`  | spans         |
| `/v1/metrics` | metric points |

## Storage

A `Monitor` keeps each entry in an open segment in memory and seals full or old segments as Parquet files in a bucket, with their catalog in a database.

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

A host of several instances passes the database's channel, so tails and searches see every instance's open entries.

## Service

Callers read an installation's entries, or a host's own entries when `installation` is absent.
`search`, `tail`, `trace` and `series` each require their own permission, such as `read-logs` for `search`.

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

Readers without the `unmask` permission see `****` in place of each attribute value under a `sensitive.` key, and each unmasked read records a `monitor.unmask` audit event.

```ts
log.info("user.invited", { "sensitive.email": invite.email, role: "editor" });
```

## Settings

`telemetryRetention` sets how many days a space's entries stay searchable, and `traceSampling` sets the share of traces kept beside every failed or slow one.

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
