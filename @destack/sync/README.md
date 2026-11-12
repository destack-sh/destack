# @destack/sync

Keep shapes of database rows current in subscribers and local copies.

## Overview

A feed reads a database's log once per commit and keeps every subscriber's queries current, and a replica copies a scope's rows into another database.

- **Query**: the rows of one logged table in some scopes, with filters, includes and aggregates.
- **Feed**: one database's log served to many subscribers; `watch` follows one query, and `subscribe` streams pages of changes.
- **Publication**: a named set of rows other databases subscribe to; `definePublication`.
- **Replica**: a copy of one scope's rows in another database; `follow` keeps it current, resuming where it stopped.
- **Prediction**: a client's changes shown on top of its replica until the server applies them.
- **Scope**: the space, account or universe a row lives in; `chain` reads the scopes around it.

```ts
import { type DatabaseConnection, defineTable, text } from "@destack/db";
import { Feed, type Query } from "@destack/sync";

const task = defineTable(
    "task",
    { id: text("id").primaryKey(), scope: text("scope").notNull(), state: text("state") },
    { log: {} },
);

async (database: DatabaseConnection, spaceId: string, signal: AbortSignal) => {
    const feed = new Feed(database, [task]);
    const open: Query = { table: task, scopes: [spaceId], where: { state: "open" } };
    for await (const items of feed.watch("open", open, signal)) {
        items.length; // 3
    }
};
```

## Examples

### Count rows by group

An aggregate query groups the rows and keeps each group's measures current.

```ts
import { defineTable, text } from "@destack/db";
import type { Query } from "@destack/sync";

const task = defineTable(
    "task",
    { id: text("id").primaryKey(), scope: text("scope").notNull(), state: text("state") },
    { log: {} },
);
const counts: Query = {
    table: task,
    scopes: ["space-1"],
    aggregate: { groupBy: ["state"], values: { tasks: { function: "count" } } },
};
```

### Copy a scope's rows

A `Replica` keeps a copy of one scope's tables in a local database, and `isSynced` tells once the first copy is complete.

```ts
import { type DatabaseConnection, defineTable, text } from "@destack/db";
import { Replica } from "@destack/sync";

const task = defineTable("task", { id: text("id").primaryKey() }, { log: {} });

async (local: DatabaseConnection) => {
    const board = new Replica({ name: "board", scope: "space-1", tables: [task] });
    const isSynced = await Replica.isSynced(local, "board", "space-1");
};
```

### Stamp a request

`RequestId.create` stamps a UUIDv7, and `RequestId.derive` derives the same UUIDv8 on every repeat.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { RequestId } from "@destack/sync";

const environment = BunEnvironment.system;
const created = RequestId.create(environment);
const derived = await RequestId.derive(Date.UTC(2026, 10, 12), "reading-1", environment.crypto);
```

## Comparison

### ElectricSQL

Shapes of rows streamed from the database's log follow ElectricSQL; four things differ.

- One feed evaluates every subscriber's queries incrementally per commit, with includes and aggregates.
- A subscriber sees only the rows its audience may read.
- A client's writes show on top of its replica until the server applies them, as Replicache does.
- A replica resumes after a lapsed credential with a fresh one, and keeps its position across restarts.
