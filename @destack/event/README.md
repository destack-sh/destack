# @destack/event

`@destack/event` keeps events in a database table, moves them into Parquet segments in a bucket, and reads, folds and forgets them.

```ts
const kind = call.eventKind();
await store.append(kind, [{ scope, id, time, keys, data }], transaction);
await store.query(kind, { scopes: [scope], where: Filter.parse('outcome = "failure"') });
await store.get(kind, { scopes: [scope], id });
await store.series(metric.eventKind(), { scopes: [scope] }, { fold: "p99", group: ["name"] });
await store.forget("user-…");
```

## Storage

A database keeps each kind's table, the segment catalog and people's keys, and the store's controllers flush, compact and expire segments.

```ts
export const database = defineDatabase({ name: "main", tables: eventTables([call.eventKind()]) });
const store = new EventStore({
    database,
    kinds: [call.eventKind()],
    files,
    personal,
    clock,
    report,
});
```

## Reading in order

`changes` reads the events appended after a log position, and an `EventReader` keeps its place in a log slot, one per key of its family.

```ts
const metering = new EventReader("metering", 7 * DAY);
await metering.at(accountId).place(now, database); // in the appending transaction
const accountIds = await metering.keys(database);
const reader = metering.at(accountId);
const after = await reader.after(database);
const { events, sequence } = await store.changes(kind, { scopes: [accountId] }, after, 100);
await reader.move(after, sequence, now, database);
```

## Tests

`EventFixture` runs a store on SQLite or PostgreSQL with a memory bucket and keyring.

```ts
const fixture = await EventFixture.open(dialect, [
    testKind({ name: "reading", keys, data, delivery: "exactly-once", policy }),
]);
await fixture.settle();
```
