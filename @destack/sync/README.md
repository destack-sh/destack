Synchronise the results of tree-shaped queries over logged tables current in subscribers, in copies held by other databases, and in local views.

## Queries

A query holds the rows of one logged table in some scopes that meet a condition, the first of an order, with what each row includes.

```ts
const board: Query = {
    table: project,
    scopes: ["space-1"],
    where: Condition.eq("archived", false),
    order: [{ column: "name", direction: "asc" }],
    limit: 50,
    include: {
        tasks: {
            table: task,
            on: { kind: "key", column: "projectId", parent: "id" },
            order: [{ column: "rank", direction: "asc" }],
            limit: 5,
        },
        size: {
            table: task,
            on: { kind: "key", column: "projectId", parent: "id" },
            aggregate: { values: { tasks: { function: "count" } } },
        },
    },
};
```

| Part | Meaning |
|---|---|
| `where` | A db `Condition` over logged columns, computed values and `exists` over relations |
| `compute` | Values computed per row, with `lookup` and `rollup` over relations |
| `order`, `limit` | A window of the first rows, per held row for an include |
| `include` | Rows a path reaches from each held row: `key`, `junction`, `descendants` or `ancestors` |
| `relations` | Related rows the condition and computed values measure, without holding them |
| `aggregate` | `count`, `sum`, `avg`, `min`, `max` per `groupBy` group, held instead of the rows |

## Dataflow

A dataflow compiles queries to one pipeline per node and keeps them current as of a log position, one run of whole transactions at a time.

```text
Dataflow                      queries → pipelines, stepped per run
├── Relation[]                measures relations filters read, deepest first
├── Selection | Aggregation   rows or groups per node, roots first
│   ├── Input                 partitions ⇄ rows: scan, key, junction, tree
│   ├── Filter                computed values, condition, visibility
│   └── Window | Tally        first rows of an order, or a group's measures
├── Sink                      rows by holder count, groups shown → Patch
└── Trace | Mirror            a copy's relations and aggregates, as its source measured them
```

| Verb | Effect |
|---|---|
| `hydrate` | Fill every pipeline as of a view, a page of a root's rows at a time |
| `step` | Apply a run of changes to what the pipelines hold |
| `fill`, `read` | Hold the queries at once and read their nested results, for a materialized dataflow |
| `inspect` | Describe each pipeline and the cost of the last and every run |

## Feeds

A feed reads a database's log once per commit and serves every subscriber from memory; streams whose queries and audience decide alike share one evaluation.

```ts
const feed = new Feed(database, [project, task]);
for await (const page of feed.subscribe({ board }, after, signal, { audience })) {
    apply(page);
}
for await (const result of feed.watch("board", board, signal)) {
    render(result);
}
```

| Option | Meaning |
|---|---|
| `subscribers`, `capacity` | The most subscribers, and the most rows and groups one evaluation knows |
| `changes` | The recent changes kept in memory, which subscribers read without the log |
| `observe` | A callback receiving each run's cost, for telemetry and slow-run logs |
| `heartbeat` | How long a caught-up stream waits before repeating its position, so copies know their source is alive |

`feed.inspect()` describes how far the feed read, what it keeps, and each shared evaluation's dataflow.

| Page field | Meaning |
|---|---|
| `reset` | A snapshot starts; drop what the subscriber holds |
| `complete` | The subscriber holds its queries at `position` |
| `changes` | Rows entering, changing or leaving, with the columns `concealed` from the subscriber |
| `results` | Aggregate groups changing, with the parts of each average |
| `outcomes` | The subscriber's mutations the page executed or rejected |
| `broadcasts` | The events sent to what the subscriber follows, which nothing stores |

## Audiences

An audience decides which rows a subscriber may hold and which columns it may read, as of the page's position with access as of now.

| Member | Meaning |
|---|---|
| `where`, `admits` | The rows the subscriber may hold, as SQL and in memory |
| `concealable`, `conceals` | The columns some rows hide, which queries may not filter, order, join or measure by |
| `watches`, `dependents` | The access tables it follows, and the rows each of their changes decides again |

## Trackers

A tracker keeps a memory database the same on every instance of a service, removing an owner's rows once no instance holds it.

```ts
const tracker = new Tracker(memory, [cursor], relay);
const rows = await memory.transaction(async (transaction) => {
    await transaction.insert(cursor).values({ id, scope, position: 12 });
    return tracker.record(transaction, () => owner);
});
tracker.publish(rows);
tracker.hold(owner); // while one of the owner's connections is open here
```

| Verb | Effect |
|---|---|
| `record`, `publish`, `end` | Send a transaction's rows to every instance, and remove an owner's rows everywhere |
| `hold`, `release`, `isHeld`, `watchHolds` | Track which owners some instance holds |
| `broadcast`, `listen` | Send an event to a topic's listeners on every instance, storing nothing |

## Replicas

A replica copies one scope's query results into another database, staging each run of pages and applying it in one transaction once complete.

```ts
const copy = new Replica({ name: "board", scope: "space-1", tables: [project, task] });
await copy.follow(database, (after, signal) => feed.subscribe({ board }, after, signal), signal);
const projects = await copy.rows(database, "board", board, outbox);
```

| Verb | Effect |
|---|---|
| `follow`, `apply` | Copy pages into the database |
| `reach`, `origins` | Wait until a copy reflects its home up to a position, and read the home position each copy reflects |
| `rows` | Read a query's rows with their includes nested, local predictions included |
| `results` | Read an aggregate query's groups with local predictions added |
| `upstream` | Measure relations and aggregates through the source's groups, for a dataflow over the copy |
| `inspect` | Describe the position, the origin, whether the tables keep the copied shape, and what the copy holds and stages |

A copy records the shape of its tables' logged columns; once a migration changes them, it snapshots again.
A database relaying its copies serves each copy's record with the rows, so its followers record the home position they reflect as their `origin`.

## Outboxes

An outbox holds a SQLite client's mutations, predicted locally, until the source executes or rejects them.

| Verb | Effect |
|---|---|
| `add` | Predict a mutation locally and append it |
| `pending`, `predicted`, `wait` | Read the mutations the source has not executed and their row changes, or wait for one |
| `acknowledge`, `reject`, `outcome` | Record and read what the source made of a mutation |
| `forget` | Remove an origin's rejected mutations |
| `revert`, `replay` | Rebase the predictions around applied pages |
| `inspect` | Count the mutations pending, executed and rejected |

## Storage

A database holding or serving copies includes `REPLICA_TABLES` and feeds them, and a client with an outbox also includes `mutation`.
