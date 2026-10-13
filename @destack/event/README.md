# @destack/event

`EventStore` keeps the events of each events type hot in SQL, flushes them into Parquet segments in the scope's bucket, and seals personal values for crypto-shredding.

```ts
// an events type's kind, built by @destack/object from its table
const kind = call.eventKind();
await store.append(kind, [{ scope, id, time, keys: { actor, method }, data }], transaction); // kept once
// one condition language over hot rows and Parquet, time and identity included
await store.query(kind, {
    scopes: [scope],
    where: Filter.parse('outcome = "failure" AND time >= 1791158400000000'),
});
await store.get(kind, { scopes: [scope], id }); // a per-segment identifier filter prunes flushed segments
await store.series(
    usage.eventKind(),
    { scopes: [scope], where },
    { measure: "quantity", fold: "sum", group: ["meter"] },
);
// p50, p90 and p99 merge the distributions a kind records, such as metric histograms
await store.series(metric.eventKind(), { scopes: [scope] }, { fold: "p99", group: ["name"] });
await store.forget("user-…"); // crypto-shredding: the person's key goes, the events stay
```

## Storage

A database keeps each kind's hot table, the segment catalog and people's keys; the store's controllers flush, compact and expire segments.

```ts
export const spaceDatabase = defineDatabase({
    name: "space",
    tables: eventTables([call.eventKind()]),
});
const store = new EventStore({
    database,
    kinds: [call.eventKind()],
    files,
    personal,
    clock,
    report,
});
loop.add(...store.controllers);
```

## Reading in order

`changes` reads appends after a log position, and an `EventReader` holds a log slot so that a reader misses no event.

```ts
const reader = new EventReader(`metering:${accountId}`, 7 * DAY);
const { events, sequence } = await store.changes(
    usage.eventKind(),
    { scopes: [accountId] },
    await reader.after(database),
    100,
);
await reader.advance(database, sequence, now);
```

## Tests

Tests run on SQLite and PostgreSQL through `EventFixture` with a memory bucket and keyring.

```ts
const fixture = await EventFixture.open(dialect, [
    testKind({ name: "reading", keys, data, delivery: "exactly-once", policy }),
]);
await fixture.settle(); // reconcile every controller once
```
