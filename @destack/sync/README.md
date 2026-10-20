# @destack/sync

`@destack/sync` copies rows from one database to another over the `@destack/db` log, the way PostgreSQL's logical replication does.

```ts
const board = definePublication({
    name: "board",
    parameters,
    audience: "reader",
    isReadinessGate: true,
    replica,
});
const query: Query = {
    table: project,
    scopes: [spaceId],
    relations,
    with: { tasks: { limit: 5 } },
};
for await (const page of feed.subscribe({ board: query }, after, signal, { audience })) send(page);
await replica.follow(local, source, signal, { subscription }); // copy the rows into a local database
await prediction.add(local, mutationId, origin, mutate); // show a client's change before the server has it
```

## Publications

`definePublication` declares a set of rows that other databases can subscribe to.

```ts
const board = definePublication({
    name: "board",
    parameters: schema.object({ board: schema.string() }),
    audience: "reader",
    isReadinessGate: true, // the subscriber serves only once it has the rows
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
    target: spaceId,
    parameters: { board: boardId },
});
```

## Queries

A `Query` selects the rows of one table in some scopes.

```ts
const open: Query = { table: task, scopes: [spaceId], where: { isDone: false } };
const counts: Query = {
    ...open,
    aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } },
};
```

## Feeds

A `Feed` reads a database's log once per commit and serves all of its subscribers.

```ts
const feed = new Feed(database, [project, task]);
for await (const rows of feed.watch("board", board, signal)) {
    render(rows);
}
```

## Positions

A stream sends its position on each heartbeat, or when a waiting subscriber asks for it with `confirm`, as Convex and ElectricSQL do.

```ts
const pages = feed.subscribe(queries, after, signal, { opened: (stream) => streams.add(stream) });
stream.confirm(position); // send a position at or past this one
await publisher.confirm(subscription, watermark); // the same, through a publisher
```

## Replicas

A `Replica` keeps a copy of one scope's rows in another database.

```ts
const board = new Replica({ name: "board", scope: spaceId, tables: [project, task] });
await board.follow(local, stream, signal, { subscription, audience, clock });
await Replica.isSynced(local, "board", spaceId); // true once the first copy is complete
await Replica.drop(local, board);
```

## Uplinks

An `Uplink` sends a subscriber's changes and queries back to where the rows live.

```ts
const uplink = Uplink.of((signer) => client.store);
await uplink.receive(mutation, signer); // the mutation's results
await uplink.call(call, scope, signer); // the query's result
```

## Predictions

A `Prediction` shows a client's changes on top of its replica until the server applies them, as Replicache does.

```ts
const prediction = new Prediction({ tables: [project, task], predict, reads, branches });
const next = await prediction.pending(local, { limit: 100 }); // the changes the next push sends
await prediction.checkout(local, branchId); // queue the next changes on a branch
```

## Scopes

`Scope` reads the scopes around a scope, and blocks writes while a scope moves.

```ts
const links = await Scope.chain(snapshot, spaceId); // nearest first
await Scope.fence(database, spaceId, targetMachine, Date.now()); // refuse writes during a move
await Scope.cap(database, spaceId, Date.now()); // refuse writes except deletes
```
