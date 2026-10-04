# @destack/sync

Keep the results of tree-shaped queries over logged tables current in subscribers, copies and local views.

## Queries

`Query` selects rows of one logged table in some scopes with db's `QueryOptions`.

```ts
const relations = defineRelations({ project, task }, (r) => ({
    project: { tasks: r.many.task({ from: r.project.id, to: r.task.projectId }) },
}));
const board: Query = {
    table: project,
    scopes: [spaceId],
    relations,
    where: { archived: false },
    orderBy: { name: "asc" },
    limit: 50,
    with: { tasks: { limit: 5 } },
};
const size: Query = {
    table: project,
    scopes: [spaceId],
    relations,
    with: { tasks: { aggregate: { values: { tasks: { function: "count" } } } } },
};
```

## Includes

`included(query, name)` resolves an include of a query as a query of the related table.

```ts
const tasks = included(board, "tasks"); // a Query over task, limited to 5 per project
```

## Query fields

`aggregate` keeps measures per group in place of a query's rows.

```ts
const query: Query = {
    table: task,
    scopes: [spaceId],
    where: { isDone: false }, // a db Condition over logged columns and computed values
    extras: { comments: { kind: "rollup", function: "count", via: "comments", where: {} } }, // values computed per row over relations
    orderBy: { createdAt: "desc" }, // the first rows, completed by the primary key, per selected row
    limit: 50,
    with: { comments: { limit: 3 } }, // the rows each named relation joins: a key, junction or tree path
    relations, // the schema's relations, which with, conditions, lookups and rollups name
};
const counts: Query = {
    table: task,
    scopes: [spaceId],
    aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } }, // kept instead of the rows
};
```

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

`Page` moves a subscriber from one log position to the next, with rows in their JSON form: exact integers as decimal text and bytes as base64.

```ts
interface Page {
    readonly reset: boolean; // a snapshot starts, so the subscriber drops what it has
    readonly complete: boolean; // the subscriber has its queries at position
    readonly position: LogPosition; // the log position to continue after
    readonly changes: readonly RowChange[]; // rows entering, changing or leaving, concealed columns named
    readonly results?: readonly ResultChange[]; // aggregate groups changing
    readonly broadcasts?: readonly Broadcast[]; // events sent to what the subscriber follows
    readonly scopes?: readonly string[]; // the scopes the subscription reads, nearest first
}
```

## Audiences

An `Audience` decides which rows a subscriber may have and which columns it may read, and `Watch.matches` tests a change against the `watches` that decide again.

```ts
feed.subscribe({ board }, after, signal, { audience: EVERYONE });
const isDeciding = Watch.matches(audience.watches, change);
```

## Dataflows

A `Dataflow` compiles queries to pipelines and keeps their results current as of a log position.

```ts
const dataflow = new Dataflow(
    { board },
    { audience, database, changesThrough: changesThroughLog(database), isMaterialized: true },
);
await dataflow.load(await View.latest(database));
const rows = await dataflow.read("board");
```

## Replicas

A `Replica` copies one scope's query results into another database.

```ts
const copy = new Replica({ name: "board", scope: spaceId, tables: [project, task] });
await copy.follow(local, ({ after }, signal) => feed.subscribe({ board }, after, signal), signal, {
    prediction,
});
const projects = await copy.rows(local, "board", board, prediction);
```

## Copies across scopes

`within` copies a table across scopes, into the scopes another copied table's rows live in.

```ts
const copy = new Replica({
    name: "work",
    scope: spaceId,
    tables: [project, note],
    within: new Map([[note, [project]]]),
});
```

## Shared rows

`Replica.includes` matches the rows a replica includes, and a row stays until no replica includes it.

```ts
const tasks = await local.select().from(task).where(Replica.includes("board", task));
```

## Dropping copies

`drop` deletes the rows only this copy includes, retracts its projections and forgets its record, and `Replica.subscriptions` lists the subscriptions a database's copies follow.

```ts
const kept = await Replica.subscriptions(local);
await copy.drop(local);
```

## Projections

`projectors` upsert a target row per kept source row by its unique source reference, and retract it when the source row leaves the copy.

```ts
const inbox = new Replica({ name: "inbox", scope: spaceId, tables: [], projectors: [entries] });
```

## Shapes

`defineShape` declares a served shape, and `subscription` builds the `Subscription` cells and clients follow it through.

```ts
const board = defineShape({
    name: "board",
    parameters: schema.object({ board: schema.string() }),
    audience: "reader",
    replica: ({ name, scope, parameters }) =>
        new Replica({
            name,
            scope,
            tables: [task],
            where: new Map([[task, { boardId: parameters.board }]]),
        }),
});
const subscription = board.subscription({
    name: "board",
    scope,
    below: spaceId,
    parameters: { board: boardId },
});
await board
    .replica(subscription)
    .follow(local, (from, signal) => source.stream({ ...subscription, ...from }, signal), signal, {
        subscription,
    });
```

## Shape audiences

`audience` names who a shape's copy is for, which admits its followers and decides its rows.

```ts
type ShapeAudience =
    | "contained" // the scope below, by containment, as a chain's access rows are
    | "reader" // the principal of the scope below where the rows live, or the caller relaying them
    | "caller" // the calling follower in the copied scope
    | "recipient"; // each row's recipient, followed only by the home they live in
```

## Resuming

`follow` resumes a completed subscription from its position, reshapes other parameters from `previous` without a refetch, and starts another shape from a snapshot.

```ts
const next = board.subscription({
    name: "board",
    scope,
    below: spaceId,
    parameters: { board: otherBoardId },
});
await board
    .replica(next)
    .follow(local, (from, signal) => source.stream({ ...next, ...from }, signal), signal, {
        subscription: next,
    });
```

## Predictions

`Prediction` shows a client's queued mutations over its copy: the main line's, then a checked-out branch's rows, then the branch's own edits.

```ts
const prediction = new Prediction({ tables: [project, task], predict, reads, branches });
const result = await prediction.add(local, mutationId, origin, async (transaction) => ({
    calls,
    result: await rename(transaction),
}));
await prediction.checkout(local, branchId); // edits now queue for the branch
```

## Acknowledgements

`acknowledge` marks a mutation executed at the source's watermark, and the copy's first completed page at that watermark drops its prediction.

```ts
const next = await prediction.pending(local, { limit: 100 }); // the mutations the next push sends
await prediction.acknowledge(local, next[0].id, watermark);
await prediction.reject(local, next[1].id, failure);
const { pending, executed, rejected } = await prediction.inspect(local);
```

## Trackers

`Tracker` keeps a memory database equal on every instance of a service and relays events between them.

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

`replicaTables` lists the tables of a database with copies, and `predictionTables` those a client queueing mutations adds.

```ts
export const local = defineDatabase({
    name: "local",
    tables: [...replicaTables, ...predictionTables, project, task],
});
```

## Scopes

`Scope` reads a scope's chain of enclosing scopes, nearest first, and fences a scope while its databases move.

```ts
import { Scope } from "@destack/sync";

const links = await Scope.chain(snapshot, spaceId);
const owner = await Scope.object(snapshot, spaceId);
await Scope.fence(database, spaceId, targetCell, Date.now()); // writes to the scope now refuse
await Scope.guard(transaction, [spaceId]); // keeps a write's scopes unfenced until it commits
await Scope.unfence(database, spaceId);
```
