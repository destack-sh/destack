# @destack/db

Declare tables, query them on SQLite and PostgreSQL, and log every commit.

## Overview

`@destack/db` is Destack's database layer: a SQL query builder with Drizzle's API on SQLite and PostgreSQL, plus a commit log, controllers, and migrations planned from your tables.

You declare your schema with tables and a database:

- `Table`: columns, constraints and indexes, declared with `defineTable`.
- `Database`: a package's tables and relations, declared with `defineDatabase` and connected with `connect…`.

You read and write with queries, conditions and transactions:

- `SelectQuery` and `MutationQuery`: built by `select`, `insert`, `update` and `delete`, as in Drizzle.
- `Condition` and `Filter`: which rows a query touches, as JSON or as a filter string.
- `Transaction`: reads one snapshot and commits its writes at once.

Logged tables let your code react to every commit:

- `Log`: every commit to a logged table, in order.
- `Controller`: updates the rows matching its `pending` condition until none match.
- `Projection`: writes rows derived from other tables, one key at a time.

Files and limits live next to the rows:

- `BlobStore`: content-addressed files that `blob` columns reference.
- `Bucket` and `Lease`: rate limits, and slots held for a while.

Declare the tables, then connect, migrate, write and read:

```ts
import { defineDatabase, defineTable, eq, identifier, text } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { BunEnvironment } from "@destack/environment/bun";
import { Identifier } from "@destack/schema";

// define a logged table and its database
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

// connect to a SQLite file and migrate it to the table
const environment = BunEnvironment.system;
const database = await connectBunSqlite("notes.db", notes, { environment });
await database.migrate([note]);

// insert a note in a transaction
const id = Identifier.create("note", environment);
const scope = Identifier.create("space", environment);
await database.transaction((transaction) =>
    transaction.insert(note).values({ id, scope, title: "Plans" }).execute(),
);

// query the notes by title
const rows = await database.select().from(note).where(eq(note.title, "Plans"));
```

## Reference

### Table

`defineTable(name, columns, options)` declares a table with the column functions below.

| Columns                                                   | Value                                                     |
| --------------------------------------------------------- | --------------------------------------------------------- |
| `text`, `integer`, `real`, `boolean`, `bigint`, `numeric` | a scalar                                                  |
| `identifier(name, prefix)`                                | a prefixed identifier, such as `note-…`                   |
| `json(name)`                                              | a JSON value, validated by a schema                       |
| `binary(name)`                                            | bytes                                                     |
| `blob(name)`, `blobs(name)`, `blobMap(name)`              | a reference to one, many or named content-addressed files |

Each column takes `.primaryKey()`, `.notNull()`, `.default(value)`, `.references(() => column, actions)`, `.sensitive()` and `.validate(schema)`.

| Option             | What it declares                                              |
| ------------------ | ------------------------------------------------------------- |
| `constraints`      | `index`, `uniqueIndex`, `unique`, `check` and `foreignKey`    |
| `log`              | that commits are logged, with their `retention`               |
| `fullText`         | the text properties a full-text index covers                  |
| `tree`             | the `id`, `scope` and `parent` properties of a tree per scope |
| `aggregates`       | counts, sums, minimums and maximums kept on referenced rows   |
| `dependents`       | rows of other tables that cascade or restrict deletion        |
| `convert`, `moved` | how an edition converts stored rows, and the previous names    |
| `predefined`       | rows every database has                                       |

### Database

`defineDatabase({ name, tables, relations })` declares a package's database, and a connection runs it.

| Verb                                                     | What it does                                  |
| -------------------------------------------------------- | --------------------------------------------- |
| `connectBunSqlite(path, database, options)`              | connects to a SQLite file in Bun              |
| `connectPostgres(url, tables, { schema, environment })`  | connects to a schema of PostgreSQL            |
| `migrate(tables)`                                        | updates the database to the declared tables   |
| `select`, `selectDistinct`, `insert`, `update`, `delete` | build queries, as in Drizzle                  |
| `query.<table>.findMany`, `findFirst`                    | read rows with their relations, as in Drizzle |
| `transaction(write)`                                     | commits writes at once                        |
| `log`                                                    | the database's `Log`                          |

### Condition

A JSON condition on a table's rows: fields map to values or comparisons, relations to conditions on their rows.
`AND`, `OR` and `NOT` combine conditions, `SEARCH` matches a full-text query, and `Condition.render` turns a condition into SQL.

| Comparison                           | Matches                                                                                        |
| ------------------------------------ | ---------------------------------------------------------------------------------------------- |
| a value, or `eq`                     | an equal value                                                                                 |
| `ne`                                 | a different value                                                                              |
| `lt`, `lte`, `gt`, `gte`             | a lower or higher value                                                                        |
| `in`, `notIn`                        | one, or none, of some values                                                                   |
| `like`, `ilike`                      | text by a pattern, `%` matching any run and `_` any one character, `ilike` ignoring ASCII case |
| `notLike`, `notIlike`                | text the pattern does not match                                                                |
| `startsWith`, `endsWith`             | text with a prefix or a suffix                                                                 |
| `characters`                         | text of only the given characters: letters, digits, their ranges, and `._+/=:@*-`              |
| `length`                             | text of a length, given as a number or the same comparisons                                    |
| `isNull`, `isNotNull`                | a missing or a present value                                                                   |
| `{ field: "name" }`                  | as the value of a comparison: another field of the row                                         |
| `now`, `now.minus(…)`, `now.plus(…)` | as the value of a comparison: the time the query runs, for controllers                         |
| `AND`, `OR`, `NOT` inside a field    | combine a field's comparisons                                                                  |

### Filter

A filter string in AIP-160's language, with SQL's precedence: `NOT` first, then `AND`, then `OR`.
`Filter.parse` reads it into a condition, and restrictions next to each other join with `AND`.

| Filter                                  | Reads as                                                                |
| --------------------------------------- | ----------------------------------------------------------------------- |
| `status = open`                         | `{ status: "open" }`                                                    |
| `title = "Ship it"`                     | `{ title: "Ship it" }`, a quoted text                                   |
| `title = 'Ship it'`                     | the same text in single quotes                                          |
| `status != done`                        | `{ status: { ne: "done" } }`                                            |
| `priority < 2`, `<=`, `>`, `>=`         | `{ priority: { lt: 2 } }`, and `lte`, `gt`, `gte`                       |
| `title:plan`                            | `{ title: { ilike: "%plan%" } }`, containment ignoring case             |
| `title = plan*`                         | `{ title: { like: "plan%" } }`, an unquoted `*` matching any run        |
| `deletedAt = null`, `deletedAt != null` | `{ deletedAt: { isNull: true } }`, `{ deletedAt: { isNotNull: true } }` |
| `isStarred = true`                      | `{ isStarred: true }`                                                   |
| `assignee = @me`                        | `{ assignee: "user-1" }`, a value the caller names in `values`          |
| `project.name = Launch`                 | `{ project: { name: "Launch" } }`, a relation                           |
| `a = 1 b = 2`, `a = 1 AND b = 2`        | `{ AND: [{ a: 1 }, { b: 2 }] }`                                         |
| `a = 1 OR b = 2`                        | `{ OR: [{ a: 1 }, { b: 2 }] }`                                          |
| `NOT a = 1`, `-a = 1`                   | `{ NOT: { a: 1 } }`                                                     |
| `(a = 1 OR a = 2) AND b = 3`            | parentheses group restrictions                                          |

`Filter.parse` takes options:

| Option         | What it does                                                                |
| -------------- | --------------------------------------------------------------------------- |
| `fields`       | refuses a field outside the set, for filters a caller writes.               |
| `values`       | gives the values `@name` stands for.                                        |
| `convert`      | converts a field's literal, such as a date for a time field.                |
| `isRelational` | reads a dotted name as one field instead of a relation, for attribute keys. |

`Filter.compile` turns a filter into a function that matches plain rows the way SQL does.
A literal's `%`, `_` and `\` match themselves.

### Transaction

A transaction reads one snapshot and commits its writes at once, refusing the commit when a row it read changed.

| Verb                                | What it does                                                         |
| ----------------------------------- | -------------------------------------------------------------------- |
| `database.commit(attempt)`          | commits an attempt, again while another commit changed a row it read |
| `Transaction.reading(database)`     | starts a transaction that only reads                                 |
| `read(table).findMany`, `findFirst` | reads rows with the transaction's writes, recording each read        |
| `count(table, condition)`           | counts rows with the transaction's writes                            |
| `write(...changes)`                 | writes rows, each as `{ table, before, after }`                      |

### Log

The log has every commit to a logged table, in order, so a reader resumes where it stopped.

| Verb                                | What it does                                                  |
| ----------------------------------- | ------------------------------------------------------------- |
| `position()`                        | reads the latest commit's position                            |
| `follow({ tables, after }, signal)` | reads pages of changes after a position, waiting for new ones |
| `at(position)`                      | reads the tables as of a position                             |

### Controller

A controller updates rows until they no longer match `pending`, and its `watches` reconcile the keys another table's changes point to.

```ts
import { defineTable, eq, text } from "@destack/db";
import { type Controller, type DatabaseConnection } from "@destack/db/database";

// define orders and their payments
const order = defineTable(
    "order",
    { id: text("id").primaryKey(), status: text("status").notNull() },
    { log: {} },
);
const payment = defineTable(
    "payment",
    { id: text("id").primaryKey(), orderId: text("order_id").notNull() },
    { log: {} },
);

/** Ship each open order, again whenever one of its payments changes. */
export const ship = (database: DatabaseConnection): Controller<typeof order> => ({
    name: "ship",
    table: order,
    when: { status: "open" },
    reads: [{ table: payment, fields: ["orderId"], on: { id: "orderId" } }],
    concurrency: 4,
    reconcile: async ([row]) => {
        // mark the order shipped
        if (row !== undefined) {
            await database.update(order).set({ status: "shipped" }).where(eq(order.id, row.id));
        }
    },
});
```

### Projection

A projection keeps rows derived from other tables up to date one key at a time, with `all` listing every key and `path` following relations from a changed row to its key.

```ts
import { defineRelations, defineTable, text } from "@destack/db";
import { type Projection } from "@destack/db/database";

// define spaces and their notes, and relate a note to its space
const space = defineTable(
    "space",
    { id: text("id").primaryKey(), account: text("account").notNull() },
    { log: {} },
);
const note = defineTable(
    "note",
    { id: text("id").primaryKey(), space: text("space").notNull() },
    { log: {} },
);
const relations = defineRelations({ space, note }, (r) => ({
    note: { space: r.one.space({ from: r.note.space, to: r.space.id }) },
}));

/** Count each account's notes across its spaces. */
export const countNotes = (write: (account: string) => Promise<void>): Projection => ({
    name: "note-usage",
    key: "account",
    all: { table: space, column: "account" },
    reads: [
        { table: space, fields: ["account"] },
        { table: note, fields: [], path: [relations.get(note, "space")] },
    ],
    result: "rows",
    project: (account) => write(account),
});
```

### Multikey index

An `index` keyed by `column.element(path, kind)` is a MongoDB multikey index, with one entry per element of a JSON array column.
`table[TABLE].entryTable(name)` reads its entries, and `.sparse()` leaves out elements without any keyed value.

```ts
import { defineTable, index, json, TABLE, text } from "@destack/db";
import { type DatabaseConnection, Transaction } from "@destack/db/database";
import { schema } from "@destack/schema";

// index each order line by customer and product
const order = defineTable(
    "order",
    {
        id: text("id").primaryKey(),
        customer: text("customer").notNull(),
        lines: json("lines", schema.array(schema.json())).notNull(),
    },
    {
        constraints: (columns) => [
            index("order_line").on(
                columns.customer,
                columns.lines.element("product.id", "text"),
                columns.lines.element("quantity", "integer"),
            ),
        ],
    },
);
const line = order[TABLE].entryTable("order_line"); // row, position, customer, productId, quantity

/** Read the lines of a customer's orders of a product. */
export const ordered = (database: DatabaseConnection, customer: string, productId: string) =>
    database.commit((transaction) =>
        transaction.read(line).findMany({ where: { customer, productId } }),
    );
```

### ControlLoop

A `ControlLoop` runs sets of controllers, projections and cursors, and starts a set once its `activation` resolves.
After a restart it continues a set's projections whose `result` is `"rows"` from the set's `position`, and lists every other job again.
A loop given the stopped loop of its process as `previous` continues that loop's sets from where they stopped.

```ts
import { ControlLoop, type DatabaseConnection, type Projection } from "@destack/db/database";

/** Run a projection once the replicas caught up, until stopped. */
export async function project(
    database: DatabaseConnection,
    projection: Projection,
    synced: (signal: AbortSignal) => Promise<void>,
    signal: AbortSignal,
) {
    // run the projection after the replicas sync
    const set = { controllers: [], projections: [projection], cursors: [], activation: synced };
    const loop = new ControlLoop(database, [{ ...set, position: "control" }]);
    void loop.run(signal);

    // wait until nothing is due among the active sets
    await loop.idle();
}
```

### BlobStore

A blob store keeps content-addressed files, each alive while a `blob` column or a lease references it.

| Verb                                                        | What it does                                            |
| ----------------------------------------------------------- | ------------------------------------------------------- |
| `put(uploads)`                                              | stores the missing files of a write in one pack, leased |
| `write(body)`, `writeJson(value)`                           | stores one file and answers its digest                  |
| `read(digest)`, `bytes(digest)`, `readJson(digest, schema)` | reads a file                                            |
| `missing(digests)`                                          | lists the files the store lacks                         |
| `fetch(digests, source)`                                    | copies the files the store lacks from another store     |
| `lease(digests, until)`                                     | keeps files until a time, before a row references them  |

### Bucket and Lease

`Bucket.take` takes tokens of a rate limit, and `Lease.acquire` takes one of a key's slots for a while.

| Verb                                                            | What it does                                          |
| --------------------------------------------------------------- | ----------------------------------------------------- |
| `Bucket.take(database, { name, key, rate, burst, now })`        | takes tokens, or answers how long until it would      |
| `Lease.acquire(database, { name, key, holder, duration, now })` | takes a slot, or answers when the earliest one lapses |
| `Lease.release(database, …)`                                    | gives a slot back                                     |

## Examples

### Connect

The same database connects to a SQLite file in Bun or to a schema of PostgreSQL.
Browsers and Durable Objects connect through their own entry points.

```ts
import { defineDatabase, defineTable, text } from "@destack/db";
import { connectBunSqlite } from "@destack/db/bun";
import { connectPostgres } from "@destack/db/postgres";
import { BunEnvironment } from "@destack/environment/bun";

// define the database
const note = defineTable("note", { id: text("id").primaryKey() });
const notes = defineDatabase({ name: "notes", tables: [note] });
const environment = BunEnvironment.system;

// connect to a SQLite file, or to PostgreSQL
const local = await connectBunSqlite("notes.db", notes, { environment });
const cloud = await connectPostgres("postgres://localhost/notes", [note], {
    schema: "notes",
    environment,
});
```

### Relate tables

Relations load a row with the rows it refers to, as in Drizzle's relational queries.

```ts
import { defineDatabase, defineRelations, defineTable, text } from "@destack/db";

// define projects and their tasks
const project = defineTable("project", { id: text("id").primaryKey() });
const task = defineTable("task", {
    id: text("id").primaryKey(),
    projectId: text("project_id").notNull(),
});

// relate a project to its tasks and a task to its project
const relations = defineRelations({ project, task }, (r) => ({
    project: { tasks: r.many.task({ from: r.project.id, to: r.task.projectId }) },
    task: { project: r.one.project({ from: r.task.projectId, to: r.project.id }) },
}));

// define the database with the tables and their relations
export const work = defineDatabase({ name: "work", tables: [project, task], relations });
```

### Replace a column by a converted one

`convert` reads each row as the previous edition stored it, so an edition can derive a column from one it drops.

```ts
import { defineTable, Expression, text } from "@destack/db";

// derive the priority from the urgent flag the second edition drops
export const task = defineTable(
    "task",
    {
        id: text("id").primaryKey(),
        title: text("title").notNull(),
        priority: text("priority").notNull(),
    },
    {
        convert: {
            2: {
                priority: Expression.case(
                    Expression.column("urgent"),
                    [{ when: 1, then: Expression.literal("high") }],
                    Expression.literal("normal"),
                ),
            },
        },
    },
);
```

The migration adds the new columns, converts the rows while the old columns still exist, then drops them.

### Rename a note in a transaction

`Transaction.run` reads a snapshot and writes each row against the value it read.
It runs again when another commit changed that row first.

```ts
import { defineTable, text } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";

// define the table
const note = defineTable("note", { id: text("id").primaryKey(), title: text("title").notNull() });

/** Rename a note, running again when another commit renamed it first. */
export async function renameNote(database: DatabaseConnection, id: string, title: string) {
    await database.commit(async (transaction) => {
        // read the note as it is
        const before = await transaction.read(note).findFirst({ where: { id } });

        // write it with the new title, against the row it read
        await transaction.write({ table: note, before: before ?? null, after: { id, title } });
    });
}
```

### Filter rows

`Filter.parse` reads a filter string into a condition, and `Condition.render` turns the condition into SQL for a table.
The [reference](#reference) has the whole filter language and every condition.

```ts
import { Condition, defineTable, Filter, integer, text } from "@destack/db";
import { type DatabaseConnection } from "@destack/db/database";

// define the table
const note = defineTable("note", {
    id: text("id").primaryKey(),
    title: text("title").notNull(),
    stars: integer("stars").notNull(),
});

/** List the notes about plans with at least three stars. */
export async function getStarredPlans(database: DatabaseConnection) {
    // read the filter into a condition, then into SQL for the table
    const condition = Filter.parse("title = plans* AND stars >= 3");
    const where = Condition.render(condition, note);

    // query the rows
    return await database.select().from(note).where(where);
}
```

### Search text

A logged table lists the properties a full-text index covers, and a `SEARCH` condition matches rows with every word of a query.

```ts
import { Condition, defineTable, text } from "@destack/db";
import { type DatabaseConnection } from "@destack/db/database";

// define a logged table whose title and body are searched
const note = defineTable(
    "note",
    { id: text("id").primaryKey(), title: text("title").notNull(), body: text("body").notNull() },
    { log: {}, fullText: ["title", "body"] },
);

/** List the notes with every word of a query. */
export async function searchNotes(database: DatabaseConnection, query: string) {
    return await database
        .select()
        .from(note)
        .where(Condition.render({ SEARCH: query }, note));
}
```

### Keep a tree

A table with a parent column per scope is a tree, and the database indexes each node's ancestors.

```ts
import { defineTable, text } from "@destack/db";

// define folders as a tree within their scope
export const folder = defineTable(
    "folder",
    {
        id: text("id").primaryKey(),
        scope: text("scope").notNull(),
        parentId: text("parent_id"),
    },
    { tree: { id: "id", scope: "scope", parent: "parentId" } },
);
```

### Aggregate referencing rows

An aggregate stores a count, sum, minimum or maximum of one table's rows on the rows they reference, kept current on every write.

```ts
import { defineTable, integer, text } from "@destack/db";

// define tasks, each in a project
const task = defineTable("task", {
    id: text("id").primaryKey(),
    projectId: text("project_id").notNull(),
});

// define projects with a count of their tasks
export const project = defineTable(
    "project",
    { id: text("id").primaryKey(), tasks: integer("tasks").notNull().default(0) },
    { aggregates: [{ from: () => task, function: "count", column: "tasks", key: "projectId" }] },
);
```

### Limit a rate and lease a slot

`Bucket.take` takes tokens of a rate limit in one statement, and `Lease.acquire` takes one of a key's slots for a while.

```ts
import { Bucket, Lease } from "@destack/db";
import { type DatabaseConnection } from "@destack/db/database";

/** Take a mail token and the sending slot of a space, at most ten mails a minute. */
export async function reserveMailSlot(database: DatabaseConnection, space: string, sender: string) {
    // take a token of the space's rate
    const now = Date.now();
    const rate = { count: 10, every: 60_000 };
    const taken = await Bucket.take(database, { name: "mail", key: space, rate, burst: 10, now });

    // lease the space's one sending slot for a minute
    const leased = await Lease.acquire(database, {
        name: "mail",
        key: space,
        holder: sender,
        duration: 60_000,
        now,
    });

    return taken.kind === "taken" && leased.kind === "acquired";
}
```

### Follow the log

`log.follow` reads pages of changes after a position and waits for new commits.

```ts
import { defineTable, text } from "@destack/db";
import { type DatabaseConnection } from "@destack/db/database";

// define a logged table
const note = defineTable("note", { id: text("id").primaryKey() }, { log: {} });

/** Print every change to the notes from now on, until stopped. */
export async function printChanges(database: DatabaseConnection, signal: AbortSignal) {
    // start after the latest commit
    const { sequence } = await database.log.position();

    // print each page of changes as commits arrive
    for await (const page of database.log.follow({ tables: [note], after: sequence }, signal)) {
        console.log(page.changes);
    }
}
```

### Run a controller

The loop runs a controller when a commit makes a row pending, so nothing polls.

```ts
import { defineTable, eq, text } from "@destack/db";
import { ControlLoop, type Controller, type DatabaseConnection } from "@destack/db/database";

// define a logged table of jobs
const job = defineTable(
    "job",
    { id: text("id").primaryKey(), status: text("status").notNull() },
    { log: {} },
);

/** Close every open job as it appears, until stopped. */
export function closeJobs(database: DatabaseConnection, signal: AbortSignal) {
    // select the open jobs, and close each one
    const close: Controller<typeof job> = {
        name: "close",
        table: job,
        when: { status: "open" },
        reconcile: async (rows) => {
            for (const row of rows) {
                await database.update(job).set({ status: "done" }).where(eq(job.id, row.id));
            }
        },
    };

    // run the controller in a loop
    const loop = new ControlLoop(database, [
        { controllers: [close], projections: [], cursors: [] },
    ]);
    void loop.run(signal);
}
```

### Encrypt a column

A `sensitive` column is encrypted at rest under its database's key, randomized by default.
A `"deterministic"` one encrypts equal values alike, so lookups by value still work, as in SQL Server's Always Encrypted.

```ts
import { defineTable, text } from "@destack/db";

// define credentials with an encrypted secret and an encrypted, searchable token
export const credential = defineTable("credential", {
    id: text("id").primaryKey(),
    secret: text("secret").notNull().sensitive(),
    token: text("token").notNull().sensitive("deterministic"),
});
```

### Test with a database

`TestDatabase.create` opens an isolated database in a test's simulation, migrated to the given tables.

```ts
import { defineTable, text } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { testSimulation } from "@destack/test/world";

// define the table and open a migrated database for the test
const note = defineTable("note", { id: text("id").primaryKey() });
await using simulation = testSimulation();
const environment = simulation.environment("test");
const storage = await TestDatabase.create([note], { environment, isMigrated: true });

// write a note, then close the database
await storage.database.insert(note).values({ id: "note-1" });
await storage.close();
```

## Comparison

### Drizzle

Tables, columns, queries and relations basically follow Drizzle v2.
Transactions rerun on conflicts, migrations are planned from your tables instead of generated files, and logged tables keep a commit log.

| Drizzle                                   | `@destack/db`                                   | Difference                                                                         |
| ----------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------- |
| `pgTable`, `sqliteTable`                  | `defineTable`                                   | One declaration for SQLite and PostgreSQL.                                         |
| `text`, `integer`, …                      | `text`, `integer`, …                            | The same, plus `identifier`, `blob` and `.sensitive()`.                            |
| `defineRelations`, `db.query`             | `defineRelations`, `database.query`             | The same.                                                                          |
| `db.select`, `insert`, `update`, `delete` | `database.select`, `insert`, `update`, `delete` | The same.                                                                          |
| `db.transaction(fn)`                      | `database.commit(fn)`                           | Reads one snapshot, and runs again when a row it read changed.                     |
| `drizzle-kit generate`, `migrate`         | `database.migrate(tables)`                      | Plans the steps from the declared tables alone, with `convert` for column changes. |
| none                                      | `Log`, `log.follow`                             | Every commit to a logged table, in order.                                          |

### Kubernetes controllers

Controllers basically follow Kubernetes: they're level-triggered and converge rows to a condition.

| Kubernetes                                       | `@destack/db`                      | Difference                                                                      |
| ------------------------------------------------ | ---------------------------------- | ------------------------------------------------------------------------------- |
| A watched resource                               | `table` and `pending`              | The rows with work are a query.                                                 |
| `Watches(...)` with `EnqueueRequestsFromMapFunc` | `watches` with `on` or a `path`    | The mapping is declared as references, so a restart replays it from the log.    |
| `resourceVersion`                                | a set's `position`                 | One position covers a set's projections whose result is rows.                   |
| `Reconcile(request)`                             | `reconcile(rows)`                  | Takes the rows of one key.                                                      |
| `RequeueAfter`                                   | a `pending` time compared to `now` | The loop wakes at the earliest due row.                                         |
| The work queue                                   | `ControlLoop`                      | Keys are deduplicated, and commits to an unlogged table wake a controller once. |
| `MaxConcurrentReconciles`                        | `concurrency`                      | The same.                                                                       |
