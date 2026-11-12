# @destack/db

Query SQLite and PostgreSQL with Drizzle's builder, and follow every committed change.

## Overview

A package declares its tables and database, and a host connects the database on Bun, PostgreSQL, a Durable Object or the browser.

- **Table**: columns, constraints and indexes, with log retention, full-text columns and release conversions; `defineTable`.
- **Database**: the tables a package keeps and replicates, with their relations; `defineDatabase`, and `migrate` plans and applies its tables.
- **Transaction**: reads at a snapshot and buffers writes; `commit` succeeds only when nothing it read changed.
- **Log**: the committed changes of logged tables in order; `follow` reads them and `at` reads the tables as of a position.
- **ControlLoop**: runs controllers, each converging the rows its `pending` condition matches, and cursors taking the log past a checkpoint.
- **BlobStore**: content-addressed blobs that `blob` columns reference, deleted once nothing references them.

```ts
import { defineDatabase, defineTable, eq, identifier, text } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { BunEnvironment } from "@destack/environment/bun";
import { Identifier } from "@destack/schema";

export const note = defineTable(
    "note",
    {
        id: identifier("id", "note").primaryKey(),
        scope: identifier("scope", "space").notNull(),
        title: text("title").notNull(),
    },
    { log: { retention: "history" } },
);
export const notes = defineDatabase({ name: "notes", tables: [note] });

const environment = BunEnvironment.system;
const database = await connectBunSqlite("notes.db", notes, { environment });
await database.migrate([note]);
const id = Identifier.create("note", environment);
const scope = Identifier.create("space", environment);
await database.transaction((transaction) =>
    transaction.insert(note).values({ id, scope, title: "Plans" }).execute(),
);
const rows = await database.select().from(note).where(eq(note.title, "Plans"));
const position = await database.log.position();
```

## Examples

### Relate tables

`defineRelations` relates a database's tables, as Drizzle's relational queries do.

```ts
import { defineDatabase, defineRelations, defineTable, text } from "@destack/db";

const project = defineTable("project", { id: text("id").primaryKey() });
const task = defineTable("task", {
    id: text("id").primaryKey(),
    projectId: text("project_id").notNull(),
});
const relations = defineRelations({ project, task }, (r) => ({
    project: { tasks: r.many.task({ from: r.project.id, to: r.task.projectId }) },
    task: { project: r.one.project({ from: r.task.projectId, to: r.project.id }) },
}));
export const work = defineDatabase({ name: "work", tables: [project, task], relations });
```

### Write in an optimistic transaction

`Transaction.run` reads at a snapshot, writes each row with its value before, and runs again when another commit changed what it read.

```ts
import { type DatabaseConnection, defineTable, text, Transaction } from "@destack/db";

const note = defineTable("note", { id: text("id").primaryKey(), title: text("title").notNull() });

async (database: DatabaseConnection) => {
    await Transaction.run(database, async (transaction) => {
        const before = await transaction.read(note).findFirst({ where: { id: "note-1" } });
        const after = { id: "note-1", title: "Ideas" };
        await transaction.write({ table: note, before: before ?? null, after });
    });
};
```

### Filter with JSON conditions

`Condition.render` turns Drizzle's relational filters, written as JSON, into SQL for a table.

```ts
import { Condition, type DatabaseConnection, defineTable, text } from "@destack/db";

const note = defineTable("note", { id: text("id").primaryKey(), title: text("title").notNull() });

async (database: DatabaseConnection) => {
    const where = Condition.render({ title: { ilike: "plan%" } }, note);
    const rows = await database.select().from(note).where(where);
};
```

### Follow committed changes

`log.follow` pages through the changes of the given tables after a position, waiting for new ones.

```ts
import { type DatabaseConnection, defineTable, text } from "@destack/db";

const note = defineTable("note", { id: text("id").primaryKey() }, { log: {} });

async (database: DatabaseConnection, signal: AbortSignal) => {
    const { sequence } = await database.log.position();
    for await (const page of database.log.follow({ tables: [note], after: sequence }, signal)) {
        page.changes.length; // 1
    }
};
```

### Run a controller

A `Controller` converges the rows its `pending` condition matches until they leave it, one key at a time.

```ts
import { ControlLoop, type Controller, type DatabaseConnection } from "@destack/db";
import { defineTable, eq, text } from "@destack/db";

const job = defineTable(
    "job",
    { id: text("id").primaryKey(), status: text("status").notNull() },
    { log: {} },
);

async (database: DatabaseConnection, signal: AbortSignal) => {
    const close: Controller<typeof job> = {
        name: "close",
        table: job,
        pending: { status: "open" },
        reconcile: async (rows) => {
            for (const row of rows) {
                const done = eq(job.id, row.id);
                await database.transaction((transaction) =>
                    transaction.update(job).set({ status: "done" }).where(done).execute(),
                );
            }
        },
    };
    const control = { controllers: [close], cursors: [] };
    const loop = new ControlLoop(database, control, { report: console.error });
    void loop.run(signal);
    await loop.idle();
};
```

### Seal a column

A `sensitive` column is encrypted at rest under its database's key, and a deterministic one can still be looked up.

```ts
import { defineTable, text } from "@destack/db";

export const credential = defineTable("credential", {
    id: text("id").primaryKey(),
    secret: text("secret").notNull().sensitive(),
    token: text("token").notNull().sensitive("deterministic"),
});
```

### Test with a database

`TestDatabase.create` opens an isolated, migrated database in a test's world.

```ts
import { defineTable, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/test/world";

const note = defineTable("note", { id: text("id").primaryKey() });
await using simulation = testSimulation();
const environment = simulation.environment("test");
const storage = await TestDatabase.create([note], { environment, isMigrated: true });
await storage.database.insert(note).values({ id: "note-1" });
await storage.close();
```

## Comparison

### Drizzle

Tables, queries, relations and inferred row types follow Drizzle; Destack's database differs in three ways.

- Writes to logged tables commit in a transaction and land in the log, which controllers, sync and replicas follow.
- Migrations come from the declared tables alone: `plan` lists each step with its risk, and a release converts columns with `convert`.
- Columns can be sealed, full-text indexed, aggregated from referencing rows, or reference content-addressed blobs.
