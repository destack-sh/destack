# @destack/event

Keep the append-only events of scopes, hot in their database and flushed into segments in their bucket, and query, fold and follow them.

## Events

An event has a `scope` keeping it, an `id` unique within its `source`, a `time` in Unix microseconds, `keys` and `data`, as CloudEvents names them.

```ts
{ scope: "organisation-…", id: "0199…", source: "space-…", time: 1791158400000000, keys: { actor, method }, data: {} }
// a copy routed to an organisation keeps the space it happened in as its source
```

## Kinds

`defineEventKind` declares a kind of event: its query keys, its data, its delivery, its policy, its route, its lock and its subject.

```ts
import { defineEventKind } from "@destack/event/declare";
import { schema } from "@destack/schema";

export const call = defineEventKind({
    name: "call",
    description: "A call made in a scope.",
    keys: schema.object({ actor: schema.string(), method: schema.string() }),
    data: schema.object({
        address: schema.sensitive(schema.string(), "personal").exactOptional(),
        token: schema.sensitive(schema.string()).exactOptional(),
    }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: HOUR, maxRows: 100_000 }, retention: 10 * YEAR },
    route: "enclosing",
    isLocked: true,
    subject: "actor",
});
```

## Delivery

An exactly-once kind's events append in the caller's transaction and are never dropped, and an at-most-once kind's events write on their own and are dropped when the write fails.

```ts
await database.transaction(async (transaction) => {
    await performTheCall(transaction);
    await store.append(
        call,
        [{ scope, id, time, keys: { actor, method }, data: { address } }],
        transaction,
    );
}); // appending an event again by its source and id changes nothing
await store.append(log, lines); // at most once: a failed write is reported and dropped
```

## Store

`EventStore` keeps the events of its kinds over a host's database and buckets, and lists the controllers the host runs.

```ts
import { DatabasePersonalKeyring, EventStore } from "@destack/event";
import { eventTables } from "@destack/event/stack";

export const spaceDatabase = defineDatabase({
    name: "main",
    tables: [...eventTables([log, span, call]), outbox],
});

const store = new EventStore({
    database,
    kinds: [log, span, call],
    files: (scope) => bucketOf(scope),
    personal: new DatabasePersonalKeyring(database, keyring),
    outbox,
    targets: (route, scope) => directory.targets(route, scope),
    deliver: (kind, events, signal) => peers.receive(kind, events, signal),
});
loop.add(...store.controllers);
```

## Queries

`query` reads a page of a scope's events in time order, hot and flushed alike, by a condition over their keys and source, text in their data and a time range.

```ts
import { Filter } from "@destack/db";

const page = await store.query(log, {
    scope,
    where: Filter.parse('severity >= 13 AND installation = "installation-…"'),
    text: "timeout",
    from,
    before,
});
const next = await store.query(log, { scope }, { after: page.cursor, limit: 100 });
```

## Export

`export` reads every page of a scope's events a filter selects.

```ts
for await (const event of store.export(call, {
    scope: organisation,
    where: Filter.parse('actor = "user-…"'),
})) {
    write(event);
}
```

## Series

`series` folds a numeric key per step and group by sum, count, min, max, average or last, hot events in SQL.

```ts
await store.series(
    usage,
    { scope, from, before },
    { measure: "quantity", fold: "sum", group: ["meter"], step: HOUR },
);
// [{ group: { meter: "db.bytes" }, steps: [{ start, value: 830000000, events: 60 }, …] }, …]
```

## Following

`tail` follows a scope's events as they commit, in commit order through the database's log, from a log sequence or from now.

```ts
for await (const { event, sequence } of store.tail(log, { scope, where }, signal)) {
    render(event);
}
```

## Flushing

A scope's hot events flush into a Parquet segment in its bucket once the oldest reaches the policy's age or the scope holds the policy's rows.

```ts
policy: { flush: { maxAge: HOUR, maxRows: 100_000 }, retention: 30 * DAY }
// the store's policy option can set either per scope
```

## Compaction and retention

Small segments of a kind without a lock are compacted into fuller ones, and segments past the retention are deleted.

```ts
isLocked: true; // locked for the retention, chained by digest, never compacted
```

## Sensitive values

An event's personal values are sealed under a key of the person its subject key names and its secrets are dropped, as `schema.sensitive` marks them, and `forget` erases the person's values in every hot and flushed event.

```ts
await store.append(call, [{ scope, id, time, keys: { actor, method }, data: { address, token } }]);
// kept: the address sealed under the actor's key; the token is never written
await store.forget("user-…"); // the events stay, their address gone
```

## Routing

A kind's route copies each event to the scopes the host's `targets` names for its scope: those enclosing it, or the account paying for it.

```ts
await store.receive(kind, events); // in the host keeping the target scope, once per event
```

## Tests

`EventFixture` keeps events over a test database, a temporary bucket and a generated keyring, on a clock the test sets.

```ts
import { EventFixture } from "@destack/event/test";

await using fixture = await EventFixture.open("sqlite", [call]);
await fixture.store.append(call, events);
fixture.now += HOUR;
await fixture.settle();
```
