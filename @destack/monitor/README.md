Store, search and follow the logs, traces and metrics of Destack scopes.

## Entries

Installations and hosts send OTLP/HTTP JSON; the monitor keeps each entry in an open segment in memory and seals full or old segments as Parquet files in a bucket.

| Path | Signal |
|---|---|
| `/v1/logs` | logs |
| `/v1/traces` | spans |
| `/v1/metrics` | metric points |

A host keeps the monitor on a database for the segment catalog and a bucket for the segments; hosts of several instances pass the database's channel, so tails and searches see every instance's open entries.

```ts
const record = AuditRecorder.service(journal, { package: monitorService.package, service: "monitor" });
const monitor = new Monitor(database, bucket, report, database.channel("monitor"));
void monitor.run(signal);
const service = implementService(monitor, { access, record });
```

## Service

Callers read an installation's entries, or a host's own entries when `installation` is absent.

| Procedure | Input | Permission |
|---|---|---|
| `search` | `EntrySearch`: an `EntryFilter` with `from`, `before` and `limit` | `read-logs` |
| `tail` | `EntryFilter`: scope, installation, severity, names, trace | `tail-logs` |
| `trace` | scope, installation, trace id | `read-traces` |
| `series` | `PointSeries`: metric name, `from`, `before`, step, grouping attributes | `read-metrics` |

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
