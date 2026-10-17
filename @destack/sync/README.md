# @destack/sync

`@destack/sync` is PostgreSQL's publications and subscriptions over the `@destack/db` log, with Zero's queries and incremental pipelines in `Query` and `Dataflow`, and Replicache's pending mutations, rebased over each server page, in `Prediction`.

```ts
// PostgreSQL's CREATE PUBLICATION
const board = definePublication({
    name: "board",
    parameters,
    audience: "reader",
    isReadinessGate: true,
    replica,
});
// Zero's related()
const query: Query = {
    table: project,
    scopes: [spaceId],
    relations,
    with: { tasks: { limit: 5 } },
};
// PostgreSQL's logical replication stream
for await (const page of feed.subscribe({ board: query }, after, signal, { audience })) send(page);
await board.follow(local, follow, signal, { prediction }); // Replicache's pull into a local database
await prediction.add(local, mutationId, origin, mutate); // Replicache's pending mutation
await prediction.acknowledge(local, mutationId, position); // Replicache's lastMutationID
```

## Publications

`definePublication` declares a publication that machines and clients subscribe to, and its `replica` replicates the publication's rows into a local database.

```ts
const board = definePublication({
    name: "board",
    parameters: schema.object({ board: schema.string() }),
    audience: "reader",
    isReadinessGate: true, // the follower serves once the replica has a snapshot
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

## Queries

A `Query` selects rows of one logged table in one or more scopes with `@destack/db`'s relational options, relations and extras, or measures them per group with `aggregate`.

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

## Replicas

A `Replica` replicates one scope's query results into another database, and keeps a row until no replica includes it.

```ts
const board = new Replica({ name: "board", scope: spaceId, tables: [project, task] });
await Replica.isSynced(local, "board", spaceId); // true once the replica took its first snapshot
await local.select().from(task).where(Replica.includes("board", task));
await board.drop(local);
// a follower's server sends changes and unhandled queries of the rows to their home
const uplink = Uplink.of((signer) => client.store);
await uplink.receive(mutation, signer); // the mutation's results
await uplink.call(call, scope, signer); // the query's result
```

## Predictions

A `Prediction` shows a client's queued mutations and checked-out branch over its replica.

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
// writes refuse while a transfer moves the scope
await Scope.fence(database, spaceId, targetMachine, Date.now());
await Scope.cap(database, spaceId, Date.now()); // writes but deletes refuse
```
