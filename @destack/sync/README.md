# @destack/sync

Keep query results current in subscribers, copies and local views.

## Queries

`Query` selects rows of one logged table in one or more scopes.

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

`aggregate` returns measures per group in place of a query's rows.

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

`Page` moves a subscriber from one log position to the next.

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

An `Audience` decides which rows and columns a subscriber may read.

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

## Copy topology

`requireAcyclic` refuses a copy back into the database a table is copied from.

```ts
await copy.requireAcyclic(database, origin); // fails with CYCLE on a copy back to its source
const follow = ({ after, origin }: Resumption, signal: AbortSignal) =>
    feed.subscribe({ board }, after, signal, origin === undefined ? {} : { origin });
await copy.follow(local, follow, signal);
const head = await source.log.position(await local.log.epoch());
```

## Copies across scopes

`within` copies a table into the scopes where another copied table's rows live.

```ts
const copy = new Replica({
    name: "work",
    scope: spaceId,
    tables: [project, note],
    within: new Map([[note, [project]]]),
});
```

## Copies of several scopes

`scopes` copies a table from several scopes.

```ts
const folders = new Replica({
    name: "folders",
    scope: Scope.universe.id,
    tables: [Scope.table, note],
    scopes: new Map([
        [Scope.table, [inboxId, archiveId]],
        [note, [inboxId, archiveId]],
    ]),
});
await Replica.reach(local, inboxId, position, signal); // the folders copy keeps the inbox
```

## Shared rows

A row stays in a copy until no replica includes it.

```ts
const tasks = await local.select().from(task).where(Replica.includes("board", task));
```

## Dropping copies

`drop` deletes a copy and the rows only it includes.

```ts
const kept = await Replica.subscriptions(local);
await copy.drop(local);
```

## Projections

`projectors` upsert one target row per source row and retract it when the source row leaves.

```ts
const inbox = new Replica({ name: "inbox", scope: spaceId, tables: [], projectors: [entries] });
```

## Shapes

`defineShape` declares a shape that cells and clients subscribe to.

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

`audience` sets who may follow a shape and which rows they get.

```ts
type ShapeAudience =
    | "contained" // the scope below, by containment, as a chain's access rows are
    | "reader" // the principal of the scope below where the rows live, or the caller relaying them
    | "caller" // the calling follower in the copied scope
    | "recipient"; // each row's recipient, followed only by the home they live in
```

## Resuming

`follow` resumes a subscription from its last position.

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

`Prediction` shows a client's queued mutations over its copy.

```ts
const prediction = new Prediction({ tables: [project, task], predict, reads, branches });
const result = await prediction.add(local, mutationId, origin, async (transaction) => ({
    calls,
    result: await rename(transaction),
}));
await prediction.checkout(local, branchId); // edits now queue for the branch
```

## Acknowledgements

`acknowledge` marks a mutation executed at the source's watermark.

```ts
const next = await prediction.pending(local, { limit: 100 }); // the mutations the next push sends
await prediction.acknowledge(local, next[0].id, watermark);
await prediction.reject(local, next[1].id, failure);
const { pending, executed, rejected } = await prediction.inspect(local);
```

## Trackers

`Tracker` keeps a memory database equal on every instance of a service.

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

`replicaTables` lists the tables of a database with copies.

```ts
export const local = defineDatabase({
    name: "local",
    tables: [...replicaTables, ...predictionTables, project, task],
});
```

## Scopes

`Scope` reads a scope's chain of enclosing scopes, nearest first.

```ts
import { Scope } from "@destack/sync";

const links = await Scope.chain(snapshot, spaceId);
const chains = await Scope.chains(snapshot, [spaceId, otherSpaceId]); // one read per level for all of them
const owner = await Scope.object(snapshot, spaceId);
await Scope.fence(database, spaceId, targetCell, Date.now()); // writes to the scope now refuse
await Scope.guard(transaction, [spaceId]); // keeps a write's scopes unfenced until it commits
await Scope.unfence(database, spaceId);
```

## Errors

A copy, subscription or scope the source cannot serve throws a `SyncError`.

```ts
import { SyncError } from "@destack/sync";

new SyncError("NOT_FOUND", "unknown scope: space-…").toServiceError(); // { code: "NOT_FOUND", … }
```
