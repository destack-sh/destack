# @destack/sync

`@destack/sync` is ElectricSQL's shapes over the `@destack/db` log, with Zero's queries and incremental pipelines in `Query` and `Dataflow`, and Replicache's pending mutations, rebased over each server page, in `Prediction`.

```ts
const board = defineShape({ name: "board", parameters, audience: "reader", isReadinessGate: true, replica }); // ElectricSQL's shape
const query: Query = { table: project, scopes: [spaceId], relations, with: { tasks: { limit: 5 } } }; // Zero's related()
for await (const page of feed.subscribe({ board: query }, after, signal, { audience })) send(page); // ElectricSQL's shape log
await copy.follow(local, follow, signal, { prediction }); // Replicache's pull into a local database
await prediction.add(local, mutationId, origin, mutate); // Replicache's pending mutation
await prediction.acknowledge(local, mutationId, position); // Replicache's lastMutationID
```

## Shapes

`defineShape` declares a shape that machines and clients subscribe to, and its `replica` copies the shape's rows into a local database.

```ts
const board = defineShape({
    name: "board",
    parameters: schema.object({ board: schema.string() }),
    audience: "reader",
    isReadinessGate: true, // the follower serves once the copy holds a snapshot
    replica: ({ name, scope, parameters }) =>
        new Replica({ name, scope, tables: [task], where: new Map([[task, { boardId: parameters.board }]]) }),
});
const subscription = board.subscription({ name: "board", scope, below: spaceId, parameters: { board: boardId } });
await board
    .replica(subscription)
    .follow(local, (from, signal) => source.stream({ ...subscription, ...from }, signal), signal, { subscription });
```

## Queries

A `Query` selects rows of one logged table in one or more scopes, with relations, extras and aggregates as in `@destack/db`'s relational queries.

```ts
const counts: Query = {
    table: task,
    scopes: [spaceId],
    where: { isDone: false },
    aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } },
};
```

## Feeds

A `Feed` reads a database's log once per commit and serves every subscriber from memory, filtered by the subscriber's `Audience`.

```ts
const feed = new Feed(database, [project, task]);
for await (const rows of feed.watch("board", board, signal)) {
    render(rows);
}
```

## Copies

A `Replica` copies one scope's query results into another database, and keeps a row until no replica includes it.

```ts
const copy = new Replica({ name: "board", scope: spaceId, tables: [project, task] });
await Replica.isSynced(local, "board", spaceId); // true once the copy took its first snapshot
await local.select().from(task).where(Replica.includes("board", task));
await copy.drop(local);
```

## Predictions

A `Prediction` shows a client's queued mutations and checked-out branch over its copy.

```ts
const prediction = new Prediction({ tables: [project, task], predict, reads, branches });
const next = await prediction.pending(local, { limit: 100 }); // the mutations the next push sends
await prediction.reject(local, next[0].id, failure);
await prediction.checkout(local, branchId); // edits now queue for the branch
```

## Scopes

`Scope` reads a scope's chain of enclosing scopes, and fences or caps the writes to it.

```ts
const links = await Scope.chain(snapshot, spaceId); // nearest first
await Scope.fence(database, spaceId, targetMachine, Date.now()); // writes refuse while a transfer moves the scope
await Scope.cap(database, spaceId, Date.now()); // writes but deletes refuse
```
