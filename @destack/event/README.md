# @destack/event

Keep the append-only events of scopes, hot in their database and sealed into their bucket, and query, fold and follow them.

## Overview

`@destack/event` is an event log after OpenTelemetry's log records: recent events in the database, older ones in Parquet segments, and folded series.

Packages declare event kinds:

- `EventKind`: one kind of event, such as a call or a span.
- `Event`: one thing that happened, with keys and data.

Hosts store and read events:

- `EventStore`: a host's events, appended, queried and followed.
- `EventFilter`: the scopes and condition of a read.
- `Series`: events folded per step and group, such as a count per hour.
- `PersonalKeyring`: a key per person for their personal values.

Declare a kind of sensor readings, then append and read them through a store:

```ts
import type { Bucket } from "@destack/bucket";
import { type DatabaseConnection, defineTable, text } from "@destack/db";
import { BunEnvironment } from "@destack/environment/bun";
import { EventKind, EventStore, eventColumns } from "@destack/event";
import { schema } from "@destack/schema";

// declare readings by sensor, flushed hourly and kept a day
const table = defineTable(
    "event_reading",
    { ...eventColumns(), sensor: text("sensor").notNull() },
    { log: { appendOnly: true } },
);
export const reading = new EventKind({
    package: import.meta.destack.package,
    name: "reading",
    table,
    keys: schema.object({ sensor: schema.string() }),
    data: schema.object({ value: schema.number() }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: 3_600_000, maxRows: 1000 }, retention: 86_400_000 },
});

/** Append a reading of a scope, and read back the newest fifty. */
export async function record(database: DatabaseConnection, bucket: Bucket, scope: string) {
    // open the store
    const { clock } = BunEnvironment.system;
    const store = new EventStore({ database, kinds: [reading], files: () => bucket, clock });

    // append the reading, then read the newest first
    const event = { scope, id: "reading-1", time: clock.now() * 1000 };
    await store.append(reading, [{ ...event, keys: { sensor: "a" }, data: { value: 21.5 } }]);

    return await store.query(reading, { scopes: [scope] }, { limit: 50, order: "descending" });
}
```

## Reference

### EventKind

`new EventKind(description)` registers a kind over a table of `eventColumns()` and query keys.
Packages declare kinds as object types with `storage: { kind: "events" }`.
A map key, such as attributes, is filtered as `<key>.<entry>`.

| Field                   | Object storage option | What it declares                                                                            |
| ----------------------- | --------------------- | ------------------------------------------------------------------------------------------- |
| `package`, `name`       | the object's          | the declaring package, and the kind's name, such as `call`                                  |
| `table`, `keys`, `data` | the object's fields   | the table of hot events, the query keys' schemas, and the data's schema                     |
| `policy.flush`          | `flush`               | when a scope's hot events flush into a segment: `maxAge` of the oldest, or `maxRows`        |
| `policy.retention`      | `retention`           | how long segments stay: milliseconds, or `{ setting }`, a package setting of days per scope |
| `delivery`              | `delivery`            | `"exactly-once"`, appended in the caller's transaction, or `"at-most-once"` outside it      |
| `isLocked`              | `isLocked`            | that segments are locked and chained for the retention, as audit trails are                 |
| `subject`               | `subject`             | the keys naming the person whose key encrypts personal values, such as an actor             |
| `distribution`          | `distribution`        | the data entry with a `Histogram` of values, which quantile folds merge                     |

### Event

An `Event` is `{ scope, id, time, keys, data }`, with `time` in Unix microseconds.
Appending the same event again changes nothing, and a changed one is refused.
Events may arrive up to five minutes late.

### EventStore

`new EventStore({ database, kinds, files, prefix, personal, settings, clock })` stores recent events in the database, and segments in each scope's bucket.
`controllers()` flush, compact and expire segments.

| Verb                                           | What it does                                                                       |
| ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| `append(kind, events, transaction)`            | appends events, in a transaction for an exactly-once kind                          |
| `query(kind, filter, { after, limit, order })` | reads a page in time order, `"ascending"` or `"descending"`, hot and flushed alike |
| `get(kind, { scopes, id })`                    | reads one event by its identity                                                    |
| `export(kind, filter)`                         | yields every selected event in time order, page by page                            |
| `series(kind, filter, request)`                | folds the selected events into series                                              |
| `changes(kind, filter, after, limit)`          | reads a page of events in commit order after a log sequence                        |
| `range(kind, filter, after, through)`          | reads the events appended between two log sequences                                |
| `forget(subject)`                              | erases a person's key, so their personal values read no more                       |

### EventFilter

A filter is `{ scopes, where }`, with a `@destack/db` condition.
It compares `scope`, `id`, `time` and query keys, and `FilterText.condition(text)` reads a filter string.

### Series

A series request is `{ measure, fold, group, step }`, answering one series per group.
Each step is `{ start, value, events }`.

| Field     | What it is                                                                                |
| --------- | ----------------------------------------------------------------------------------------- |
| `measure` | the key folded, numeric for arithmetic folds, needed by all but `count` and the quantiles |
| `fold`    | `sum`, `count`, `min`, `max`, `average`, `last`, `unique`, `p50`, `p90` or `p99`          |
| `group`   | the keys or map entries whose values each get a series of their own, or `scope`           |
| `step`    | the width of each step in milliseconds, one step over the whole range when absent         |

## Examples

### Count calls per hour

`series` counts a day's calls per hour and method.

```ts
import type { EventKind, EventStore } from "@destack/event";

/** Count a scope's calls of the last day, per hour and method. */
export async function callsPerHour(store: EventStore, kind: EventKind, scope: string, now: number) {
    // select the last day's calls, in microseconds
    const since = (now - 86_400_000) * 1000;
    const filter = { scopes: [scope] as const, where: { time: { gte: since } } };

    return await store.series(kind, filter, { fold: "count", group: ["method"], step: 3_600_000 });
}
```

### Follow events in commit order

`changes` reads the events after a log sequence.

```ts
import type { Event, EventKind, EventStore } from "@destack/event";

/** Read the next events a scope appended, answering where to continue. */
export async function readNext(
    store: EventStore,
    kind: EventKind,
    scope: string,
    after: number,
): Promise<{ readonly events: readonly Event[]; readonly sequence: number }> {
    return await store.changes(kind, { scopes: [scope] }, after, 100);
}
```

### Test with an event store

`TestEventStore` runs a store in a simulation.

```ts
import { TestEventStore, testKind } from "@destack/event/test";
import { schema } from "@destack/schema";

// declare a kind of sensor readings, kept a day
const reading = testKind({
    package: import.meta.destack.package,
    name: "reading",
    keys: schema.object({ sensor: schema.string() }),
    data: schema.object({ value: schema.number() }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: 3_600_000, maxRows: 1000 }, retention: 86_400_000 },
});

// append a reading, and run the controllers until they idle
await using fixture = await TestEventStore.open([reading]);
const event = { scope: "space-1", id: "reading-1", time: Date.UTC(2026, 9, 1) * 1000 };
await fixture.store.append(reading, [{ ...event, keys: { sensor: "a" }, data: { value: 1 } }]);
await fixture.simulation.idle();
```

## Comparison

### OpenTelemetry logs

Events follow OpenTelemetry's log records, and distributions its exponential histograms.

| OpenTelemetry            | `@destack/event`                   | Difference                                                              |
| ------------------------ | ---------------------------------- | ----------------------------------------------------------------------- |
| `LogRecord`              | `Event`                            | Keyed by its declared query keys, in one scope.                         |
| attributes               | a map key, read as `<key>.<entry>` | The same.                                                               |
| `ExponentialHistogram`   | `Histogram`                        | The same, merged by the `p50`, `p90` and `p99` folds.                   |
| an exporter to a backend | `EventStore`                       | Stored in the scope's database, then in Parquet segments in its bucket. |
| none                     | `delivery`, `forget`               | Exactly once in the caller's transaction, and erasable personal values. |
