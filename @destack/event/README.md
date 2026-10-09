# @destack/event

An `Event` is a CloudEvent (`id`, `source`, `time`, `data`) kept by a scope, and `EventStore` keeps each kind's events hot in SQL, flushes them into Parquet segments in the scope's bucket, and seals personal values for crypto-shredding.

```ts
const event: Event = { scope, id, source: spaceId, time: 1791158400000000, keys: { actor, method }, data: {} }; // CloudEvents' id, source, time, data
await store.append(call, [event], transaction); // in the caller's transaction, kept once
await store.query(log, { scope, where: Filter.parse("severity >= 13"), text: "timeout", from, before }); // AIP-160 over hot rows and Parquet
await store.series(usage, { scope, from, before }, { measure: "quantity", fold: "sum", group: ["meter"], step: HOUR });
await store.forget("user-…"); // crypto-shredding: the person's key goes, the events stay
```

## Kinds

`defineEventKind` declares a kind of event with its query keys, data, delivery, flush and retention policy, route, lock and subject.

```ts
import { defineEventKind } from "@destack/event/declare";

export const call = defineEventKind({
    name: "call",
    description: "A call made in a scope.",
    keys: schema.object({ actor: schema.string(), method: schema.string() }),
    data: schema.object({ address: schema.sensitive(schema.string(), "personal").exactOptional() }),
    delivery: "exactly-once",
    policy: { flush: { maxAge: HOUR, maxRows: 100_000 }, retention: 10 * YEAR },
    route: "enclosing", // copied to every enclosing scope
    isLocked: true, // chained by digest for the retention, never compacted
    subject: "actor",
});
```

## Store

`EventStore` keeps the events of its kinds over a database and the scopes' buckets, and lists the controllers that flush, compact and route them.

```ts
import { eventTables } from "@destack/event/stack";

export const spaceDatabase = defineDatabase({ name: "space", tables: [...eventTables([log, span, call]), outbox] });
const store = new EventStore({ database, kinds: [log, span, call], files, personal, outbox, targets, deliver });
loop.add(...store.controllers);
```

## Following

`tail` follows a scope's events in commit order, and an `EventReader` holds a log slot so that a reader misses no event.

```ts
for await (const { event } of store.tail(log, { scope, where }, signal)) {
    render(event);
}
const reader = new EventReader(`endpoint:${id}`, 7 * DAY);
const { events, sequence } = await store.changes(call, { scope }, await reader.after(database), 100);
await reader.advance(database, sequence, now);
```

## Service

`serveEvents` serves a scope's events of each kind whose `access` names its readers, and masks personal values unless the caller may unmask them.

```ts
import { serveEvents } from "@destack/event/server";

const procedures = serveEvents({ store, admit, unmasked: (context, read) => server.unmasked(context, read) });
```

## Tests

`EventFixture` keeps events over a test database, a memory bucket and a generated keyring, on a clock the test sets.

```ts
await using fixture = await EventFixture.open("sqlite", [call]);
await fixture.store.append(call, events);
fixture.now += HOUR;
await fixture.settle();
```
