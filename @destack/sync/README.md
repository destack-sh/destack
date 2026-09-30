# @destack/sync

Keep the results of tree-shaped queries over logged tables current in subscribers, copies and local views.

## Queries

A `Query` selects rows of one logged table in some scopes, with the rows each one includes.

```ts
const board: Query = {
    table: project,
    scopes: [spaceId],
    where: Condition.eq("archived", false),
    order: [{ column: "name", direction: "asc" }],
    limit: 50,
    include: {
        tasks: { table: task, on: { kind: "key", column: "projectId", parent: "id" }, limit: 5 },
        size: {
            table: task,
            on: { kind: "key", column: "projectId", parent: "id" },
            aggregate: { values: { tasks: { function: "count" } } },
        },
    },
};
```

## Query fields

Each field of a query narrows, extends or measures its rows.

| Field | Meaning |
|---|---|
| `where` | A db `Condition` over logged columns and computed values |
| `compute` | Values computed per row, with `lookup` and `rollup` over relations |
| `order`, `limit` | A window of the first rows, per held row for an include |
| `include` | Rows a `key`, `junction`, `descendants` or `ancestors` path reaches from each held row |
| `relations` | Related rows the condition measures without holding them |
| `aggregate` | `count`, `sum`, `avg`, `min` and `max` per `groupBy` group, held instead of the rows |

## Feeds

A `Feed` reads a database's log once per commit and serves every subscriber from memory.

```ts
const feed = new Feed(database, [project, task]);
for await (const page of feed.subscribe({ board }, after, signal, { audience })) {
    send(page);
}
for await (const rows of feed.watch("board", board, signal)) {
    render(rows);
}
```

## Pages

A `QueryPage` moves a subscriber from one log position to the next.

| Field | Meaning |
|---|---|
| `reset` | A snapshot starts, so the subscriber drops what it holds |
| `complete` | The subscriber holds its queries at `position` |
| `changes` | Rows entering, changing or leaving, with the columns concealed from the subscriber |
| `results` | Aggregate groups changing |
| `outcomes` | The subscriber's mutations the source executed or rejected |
| `broadcasts` | Events sent to what the subscriber follows |

## Audiences

An `Audience` decides which rows a subscriber may hold and which columns it may read, and `EVERYONE` admits all.

```ts
feed.subscribe({ board }, after, signal, { audience: EVERYONE });
```

## Dataflows

A `Dataflow` compiles queries to pipelines and keeps their results current as of a log position.

```ts
const dataflow = new Dataflow({ board }, { audience, database, changesThrough: changesThroughLog(database), isMaterialized: true });
await dataflow.fill(await View.latest(database));
const rows = await dataflow.read("board");
```

## Replicas

A `Replica` copies one scope's query results into another database.

```ts
const copy = new Replica({ name: "board", scope: spaceId, tables: [project, task] });
await copy.follow(local, (after, signal) => feed.subscribe({ board }, after, signal), signal, { outbox });
const projects = await copy.rows(local, "board", board, outbox);
```

A table in `within` is copied across scopes: its rows live in the scopes that another copied table's rows are.

```ts
const copy = new Replica({ name: "work", scope: spaceId, tables: [project, note], within: new Map([[note, [project]]]) });
```

Several replicas may include one row, which stays until none includes it and keeps a column while one of them shows it.

```ts
const tasks = await local.select().from(task).where(Replica.includes("board", task));
```

A database copying another server's rows sends a `ReplicaRequest` to a `ReplicaSource`, and resumes only the request it completed.

```ts
const request = { name: "chain", scope: accountId, below: spaceId, access: true, held: [], copied: [], rows: [] };
await copy.follow(local, (after, signal) => source.stream({ ...request, after }, signal), signal, { request });
```

## Outboxes

An `Outbox` holds a client's mutations, predicted locally, until the source executes or rejects them.

```ts
const outbox = new Outbox([project, task], predict, reach);
const result = await outbox.add(local, mutationId, origin, async (transaction) => ({ calls, result: await rename(transaction) }));
await outbox.checkout(local, "draft");
await outbox.merge(local, "draft");
const next = await outbox.pending(local, { limit: 100 });        // the mutations the next push carries
const { pending, executed, rejected } = await outbox.inspect(local);
```

## Trackers

A `Tracker` keeps a memory database the same on every instance of a service, and relays events between them.

```ts
const tracker = new Tracker(memory, [cursor], relay);
const rows = await memory.transaction(async (transaction) => {
    await transaction.insert(cursor).values({ id, scope, position: 12 });
    return tracker.record(transaction, () => owner);
});
tracker.publish(rows);
tracker.broadcast(topic, event);
```

## Storage

A database holding copies includes `replicaTables`, and a client with an outbox also includes `outboxTables`.

```ts
export const local = defineDatabase({ name: "local", tables: [...replicaTables, ...outboxTables, project, task] });
```
