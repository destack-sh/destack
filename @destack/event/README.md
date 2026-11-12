# @destack/event

Keep the append-only events of scopes, hot in their database and sealed into their bucket, and query, fold and follow them.

## Overview

An event store keeps each kind's recent events in a database table and moves older ones into Parquet segments in the scope's bucket.

- **EventKind**: one kind of event, such as a call or a log record: its query keys, data schema, delivery and retention; segments of a locked kind are chained as audit trails are.
- **Event**: one thing that happened in a scope at a time, with its keys and data; appending it again while it is hot changes nothing.
- **EventStore**: the events of a host's scopes; `append`, `query`, `get`, `series`, `changes` and `forget`, with controllers that flush, compact and expire segments.

```ts
import type { EventKind, EventStore } from "@destack/event";

async (store: EventStore, kind: EventKind, scope: string) => {
    const time = Date.now() * 1000;
    await store.append(kind, [{ scope, id: "event-1", time, keys: {}, data: null }]);
    const page = await store.query(kind, { scopes: [scope] }, { limit: 50 });
    const event = await store.get(kind, { scopes: [scope], id: "event-1" });
    const counts = await store.series(kind, { scopes: [scope] }, { fold: "count" });
};
```

## Examples

### Store events

A store keeps its kinds' tables in its database, and each scope's segments in the bucket `files` returns.

```ts
import { EventStore, type EventKind } from "@destack/event";
import type { Bucket } from "@destack/bucket";
import type { DatabaseConnection } from "@destack/db";
import { BunEnvironment } from "@destack/environment/bun";

(database: DatabaseConnection, kinds: readonly EventKind[], bucket: Bucket) =>
    new EventStore({ database, kinds, files: () => bucket, clock: BunEnvironment.system.clock });
```

### Read events in order

`changes` reads the events appended after a log position, so each reader keeps its place.

```ts
import type { EventKind, EventStore } from "@destack/event";

async (store: EventStore, kind: EventKind, scope: string, after: number) => {
    const { events, sequence } = await store.changes(kind, { scopes: [scope] }, after, 100);
};
```

### Forget a person

`forget` erases a person's key, so their personal values in every sealed segment can no longer be read.

```ts
import type { EventStore } from "@destack/event";

async (store: EventStore) => {
    await store.forget(["user-0199…"]);
};
```

### Test with an event store

`TestEventStore` runs a store and its controllers in a world, with a memory bucket.

```ts
import { schema } from "@destack/schema";
import { TestEventStore, testKind } from "@destack/event/test";

const reading = testKind({
    package: import.meta.destack.package,
    name: "reading",
    keys: schema.object({ sensor: schema.string() }),
    data: schema.object({ value: schema.number() }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: 3_600_000, maxRows: 1000 }, retention: 86_400_000 },
});
await using fixture = await TestEventStore.open([reading]);
await fixture.simulation.idle();
```

## Comparison

### OpenTelemetry logs

Events with keys and data follow OpenTelemetry's log records; three things differ.

- Recent events stay in the scope's database, so an append can join the transaction that caused it, and older ones move into Parquet segments.
- Personal values are sealed under the person's key, and `forget` erases the key.
- Each kind sets its delivery, exactly once or at most once, and its retention, fixed or read from a scope's setting.
