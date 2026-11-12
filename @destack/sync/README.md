# @destack/sync

Synchronize database rows to subscribers and local replicas, as each commit lands.

## Overview

`@destack/sync` is a sync engine after ElectricSQL: queries over logged tables, a feed that updates them per commit, and local replicas with predicted writes.

Subscribers describe what they see:

- `Query`: the rows of one logged table in some scopes, with a condition, order, limit, includes and aggregates.

Servers serve queries:

- `Feed`: a database's log, served to every subscriber.
- `Publication`: rows other databases subscribe to, declared with `definePublication`.

Clients keep local copies:

- `Replica`: a copy of one scope's rows in a local database.
- `Prediction`: a client's queued writes, shown over its replica.
- `RequestId`: a request's idempotency key.

Watch the open tasks of a space as commits land:

```ts
import { type DatabaseConnection, defineTable, text } from "@destack/db";
import { Feed } from "@destack/sync";

// define a logged table of tasks
const task = defineTable(
    "task",
    { id: text("id").primaryKey(), scope: text("scope").notNull(), state: text("state").notNull() },
    { log: {} },
);

/** Print the open tasks of a space each time they change, until stopped. */
export async function printOpenTasks(
    database: DatabaseConnection,
    space: string,
    signal: AbortSignal,
) {
    // watch the query
    const feed = new Feed(database, [task]);
    const query = { table: task, scopes: [space], where: { state: "open" } };
    for await (const items of feed.watch("open", query, signal)) {
        console.log(items.map((item) => item.row));
    }
}
```

## Reference

### Query

A query is a relational read of one logged table, as in `@destack/db`'s `findMany`.
Each result is an `Item`: `{ row, with, measures }`.

| Field               | What it is                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------ |
| `table`             | the logged table                                                                                 |
| `scopes`            | the scopes whose rows it reads, or `"every"`                                                     |
| `where`             | a `Condition` on the rows, their extras and relations                                            |
| `orderBy`, `limit`  | the order, completed by the key, and the most rows                                               |
| `columns`, `extras` | the fields each row has, and values computed from them                                           |
| `with`              | the related rows each row includes, by relation                                                  |
| `aggregate`         | `{ groupBy, values }` stored instead of rows, each value a `count`, `sum`, `avg`, `min` or `max` |
| `relations`         | the relations `with` and `where` follow                                                          |
| `permission`        | a permission the subscriber also needs on each row                                               |

### Feed

`new Feed(database, tables, options)` serves a database's logged tables.
It reads the log once per commit for all subscribers.

| Verb                                | What it does                                                         |
| ----------------------------------- | -------------------------------------------------------------------- |
| `watch(name, query, signal)`        | follows one query's items until stopped                              |
| `watchResults(name, query, signal)` | follows one aggregate query's groups                                 |
| `subscribe(queries, …)`             | follows queries from a position for an audience, as pages of changes |
| `declare(tables)`                   | serves more logged tables                                            |
| `changes(watches, sequence)`        | reads the changes after a sequence, absent once compacted            |

### Publication and Replica

`definePublication({ name, parameters, audience, replica })` declares rows other databases subscribe to.
Its audience is `"contained"`, `"reader"` or `"caller"`.

`new Replica({ name, scope, tables })` declares a replica.

| Verb                                        | What it does                                                          |
| ------------------------------------------- | --------------------------------------------------------------------- |
| `register(database, options)`               | records the replica before its first snapshot                         |
| `follow(database, source, signal, options)` | applies a source's pages until stopped, resuming where it stopped     |
| `rows(database, name, query)`, `results(…)` | reads a query's rows or groups from the replica, predictions included |
| `wait(database, position, signal)`          | waits until the replica reflects its home up to a position            |
| `Replica.isSynced(database, name, scope)`   | reports whether the replica is a complete snapshot                    |
| `Replica.drop(database, replica)`           | deletes the replica's rows and forgets it                             |

### Prediction

A prediction applies queued writes over the replica, and replays them as the server's pages arrive.

| Verb                           | What it does                                           |
| ------------------------------ | ------------------------------------------------------ |
| `add(…)`                       | predicts and queues a mutation                         |
| `pending(database)`            | reads the mutations waiting for the server, in order   |
| `acknowledge(…)`, `reject(…)`  | records the server's outcome of a mutation             |
| `replay(transaction, settled)` | settles what a page covers and predicts the rest again |
| `checkout(database, branch)`   | shows a branch over the main line                      |

### RequestId

A request identifier is a UUIDv7, valid for retries for a week.
`RequestId.create(environment)` creates one, and `RequestId.derive(time, key, crypto)` derives a repeatable one.

## Examples

### Watch projects with their tasks

`with` includes related rows through relations, each list in its own order.

```ts
import { type DatabaseConnection, defineRelations, defineTable, integer, text } from "@destack/db";
import { Feed, type Query } from "@destack/sync";

// define logged projects and their ranked tasks
const project = defineTable(
    "project",
    { id: text("id").primaryKey(), scope: text("scope").notNull(), name: text("name").notNull() },
    { log: {} },
);
const task = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        projectId: text("project_id"),
        title: text("title"),
        rank: integer("rank").notNull(),
    },
    { log: {} },
);
const relations = defineRelations({ project, task }, (relate) => ({
    project: { tasks: relate.many.task({ from: relate.project.id, to: relate.task.projectId }) },
}));

/** Print each project's tasks in rank order as they change, until stopped. */
export async function printProjects(
    database: DatabaseConnection,
    space: string,
    signal: AbortSignal,
) {
    // watch the projects by name, each with its tasks by rank
    const feed = new Feed(database, [project, task]);
    const query = {
        table: project,
        scopes: [space],
        relations,
        orderBy: { name: "asc" },
        with: { tasks: { orderBy: { rank: "asc" } } },
    } satisfies Query;
    for await (const items of feed.watch("projects", query, signal)) {
        console.log(items.map((item) => [item.row["name"], item.with["tasks"]?.length]));
    }
}
```

### Count tasks by state

An aggregate query groups rows and measures each group.

```ts
import { defineTable, text } from "@destack/db";
import type { Query } from "@destack/sync";

// define a logged table of tasks
const task = defineTable(
    "task",
    { id: text("id").primaryKey(), scope: text("scope").notNull(), state: text("state").notNull() },
    { log: {} },
);

// count the tasks of a space by state
export const tasksByState: Query = {
    table: task,
    scopes: ["space-1"],
    aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } },
};
```

## Comparison

### ElectricSQL

Queries streamed from the log basically follow ElectricSQL's shapes.
Queries also have includes and aggregates, and each subscriber only gets rows it may read.

| ElectricSQL      | `@destack/sync` | Difference                                                        |
| ---------------- | --------------- | ----------------------------------------------------------------- |
| a shape          | `Query`         | Adds includes, order, limits and aggregates over relations.       |
| the shape log    | `Feed`          | Evaluates every subscriber's queries once per commit.             |
| a shape's offset | a log position  | A replica resumes from it after a restart or a lapsed credential. |
| none             | `Prediction`    | Shows a client's writes at once, as Replicache does.              |
