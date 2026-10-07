# @destack/db

Declare, query, log and migrate SQL tables on SQLite and PostgreSQL.

## Queries

`@destack/db` exports the [Drizzle](https://orm.drizzle.team/) query builders and operators.

```ts
import { and, asc, eq, isNull, sql } from "@destack/db";

const notes = await database
    .select({ id: note.id, title: note.title })
    .from(note)
    .where(and(eq(note.scope, spaceId), isNull(note.archivedAt)))
    .orderBy(asc(note.title))
    .limit(50); // { id: Identifier<"note">; title: string }[]

await database.insert(note).values(row).onConflictDoUpdate({ target: note.id, set: { title } });
const [removed] = await database.delete(note).where(eq(note.id, id)).returning();
```

## Differences from Drizzle

`alias`, `Select`, `Insert`, `Key`, `.primaryKey()` and `execute` differ from Drizzle.

```ts
// alias a table, and name the alias with from(table) where a query reads it
const parent = alias(note, "parent");

// type rows as $inferSelect and $inferInsert do, and the key by its columns
type NoteRow = Select<typeof note>;
type NoteInsert = Insert<typeof note>;
type NoteKey = Key<typeof note>;

// mark each key column with .primaryKey(), in declaration order
const id = identifier("id", "note").primaryKey();

// parse each row of a raw statement by a schema
const rows = await database.execute(
    sql`SELECT ${note.title} FROM ${note}`,
    schema.object({ title: schema.string() }),
);
```

## Typed rows

Each column parses the values it reads with its schema.

```ts
const [row] = await database.execute(
    sql`SELECT count(*) AS total FROM ${note}`,
    schema.object({ total: schema.number() }),
);
```

## Relations

`defineRelations` declares each table's relations as in Drizzle's relational queries v2.

```ts
export const relations = defineRelations({ project, task, tag, taskTag }, (r) => ({
    project: { tasks: r.many.task({ from: r.project.id, to: r.task.projectId }) },
    task: {
        project: r.one.project({ from: r.task.projectId, to: r.project.id }),
        tags: r.many.tag({
            from: r.task.id.through(r.taskTag.taskId),
            to: r.tag.id.through(r.taskTag.tagId),
        }),
    },
}));
export const work = defineDatabase({
    name: "work",
    tables: [project, task, tag, taskTag],
    relations,
});

const database = work.get(context);
const projects = await database.query.project.findMany({
    columns: { name: true },
    where: { tasks: { isDone: false } },
    extras: { open: { kind: "rollup", function: "count", via: "tasks", where: { isDone: false } } },
    with: { tasks: { orderBy: { rank: "asc" }, limit: 3, with: { tags: true } } },
}); // { name: string; open: Scalar; tasks: { …; tags: { … }[] }[] }[]
const first = await database.query.task.findFirst({ where: { title: "Plan" } });
```

## Relation reads

`with` includes the rows of a relation.

```ts
// relate a tree table to its descendants and ancestors through its index, without declaring them
await database.query.folder.findMany({ with: { descendants: true, ancestors: true } });

// apply a relation's where wherever the relation is read
const open = r.many.task({ from: r.project.id, to: r.task.projectId, where: { isDone: false } });

// read one statement per level in one read transaction, with a limit per parent
await database.query.project.findMany({ with: { tasks: { limit: 3 } } });

// name relations in conditions, and look up a related row's column or roll up related rows in extras
await database.query.task.findMany({
    where: { project: { name: "Launch" } },
    extras: { projectName: { kind: "lookup", via: "project", column: "name" } },
});
```

## Namespaces

`relations.namespace` resolves relation names in conditions and extras.

```ts
const namespace = relations.namespace(task, {
    tagCount: { kind: "rollup", function: "count", via: "tags", where: {} },
});
await database
    .select()
    .from(task)
    .where(Condition.render({ tags: { label: "urgent" }, tagCount: { gte: 2 } }, task, namespace));
```

## Tables

`defineTable` declares a table's columns, constraints, log retention and release conversions.

```ts
export const note = defineTable(
    "note",
    {
        id: identifier("id", "note").primaryKey(),
        scope: identifier("scope", "space").notNull(),
        title: text("title").notNull(),
    },
    {
        constraints: (note) => [index("note_scope").on(note.scope)],
        log: { retention: "history" },
        convert: {
            "2026.10.0": {
                title: Expression.coalesce(Expression.column("title"), Expression.literal("")),
            },
        },
    },
);
```

## Described tables

`Table.describe` builds the table a declared state describes, the inverse of `describeTable`, naming its columns by the properties given.

```ts
const [state] = main.state().tables.sqlite;
const table = Table.describe(state, { created_at: "createdAt" }); // SQL name to property, the SQL name by default
```

## Text order

Text compares by its UTF-8 bytes on every dialect.

```sql
ALTER TABLE "note" ALTER COLUMN "title" TYPE text COLLATE "C"
```

## Databases

`defineDatabase` declares the tables a service owns and the tables it copies.

```ts
export const main = defineDatabase({ name: "main", tables: [note], copies: [accountTable] });

const database = main.get(context);
const unapplied = await main.check(database);
const isCopied = database.copies(accountTable);
```

## Providers

`databaseProvider` manages a host's databases.

```ts
import { databaseProvider } from "@destack/db";
import { BunSqliteDatabaseHost } from "@destack/db/bun";

const host = new BunSqliteDatabaseHost(new URL("file:///var/destack/databases/"));
const provider = databaseProvider(host, databaseObject);
await using connection = await main.connectors["sqlite"]?.connect(binding, main); // SQLite on Bun
```

## Database snapshots

`DatabaseHandle.snapshot` copies a database's rows into the content-addressed `ContentStore` it is given.

```ts
const digest = await DatabaseHandle.snapshot(handle, store, wrap); // rows rewritten on the way, such as host-bound keys
await DatabaseHandle.restore(target, digest, store, unwrap);
await provider.snapshot.restore(record, desired, digest, store); // a database resource
```

## Connections

`connectBunSqlite`, `connectPostgres` and `connectDurableObject` open a database on Bun, PostgreSQL and a Durable Object's storage.

```ts
import { connectBunSqlite } from "@destack/db/bun";
import { connectPostgres } from "@destack/db/postgres";
import { connectDurableObject } from "@destack/db/cloudflare";

const file = await connectBunSqlite("notes.db", main);
const pool = await connectPostgres(process.env.DATABASE_URL, main);
const storage = connectDurableObject(state.storage, main);
```

## Shared stores

`namespace` keeps a database's tables, log and state apart in a SQLite store several databases share.

```ts
const space = connectDurableObject(storage, spaceDatabase, { namespace: "space" });
const vault = connectDurableObject(storage, vaultDatabase, { namespace: "vault" });
```

## Durable Objects

`DurableObjectDatabaseHost` keeps a host's databases in one Durable Object's storage, which `durableObjectConnector` opens inside a workload.

```ts
import { DurableObjectDatabaseHost, durableObjectConnector } from "@destack/db/cloudflare";

const host = new DurableObjectDatabaseHost(state.storage);
const connector = durableObjectConnector(state.storage);
```

### Rows read and written

`DurableObjectDatabaseHost.measure` counts the bytes of an object's databases and takes the rows its clients read and wrote since the last measure, as Cloudflare bills them.

```ts
host.measure(); // { bytes: 65536, rowsRead: 3, rowsWritten: 8 }, counted per object in memory
host.measure(); // { bytes: 65536, rowsRead: 0, rowsWritten: 0 }
```

## Sole writers

A pool that writes its database alone connects as the `sole` writer without `LISTEN`.

```ts
import { connectPostgres, postgresConnector } from "@destack/db/postgres";

const only = await connectPostgres(url, main, "sole");
await using placed = await postgresConnector.connect(
    { reference: env.DATABASE.connectionString },
    main,
);
```

## Writes

`transaction` runs queries in a transaction, and `rehearse` runs one and rolls it back.

```ts
const database = await connectBunSqlite("notes.db", main);
await database.transaction(async (transaction) =>
    transaction.update(note).set({ title: "Changed" }).where(eq(note.id, id)),
);
await database.upsert(note, rows);
await database.remove(note, keys);
const plan = await database.rehearse((transaction) => transaction.plan(state)); // rolled back
```

### Retries

An outermost transaction that loses a serialization race runs again up to five times with jittered backoff, so its callback does database work alone.

```ts
await database.transaction(async (transaction) => {
    const [row] = await transaction.select().from(tally).where(eq(tally.name, "a"));
    await transaction
        .update(tally)
        .set({ value: row.value + 1 })
        .where(eq(tally.name, "a"));
});
```

## Statements

A `Statement` renders its SQL once per database and runs it with named values.

```ts
const notes = new Statement(
    (value) => sql`SELECT ${note.id} AS id FROM ${jsonElements(value("ids"), "listed")}
    JOIN ${note} ON ${note.id} = listed.value ->> 0`,
);
await notes.all(database, { ids: JSON.stringify(ids.map((id) => [id])) });
```

## Conditions

A `Condition` is plain JSON in the shape of Drizzle's relational filters.

```ts
const where: Condition<Select<typeof note>> = {
    scope: spaceId,
    OR: [{ title: { ilike: "travel%" } }, { title: { in: ["Inbox", "Ideas"] } }],
    NOT: { title: "Draft" },
};
await database.select().from(note).where(Condition.render(where, note));
```

## Filters

`Filter.parse` reads an AIP-160 filter into a condition.

```ts
Filter.parse('status = open AND (due < @friday OR priority = high) -title:"draft"', {
    fields: new Set(["status", "due", "priority", "title"]),
    values: { friday: fridayAt },
});
// { AND: [{ status: "open" }, { OR: [{ due: { lt: fridayAt } }, { priority: "high" }] }, { NOT: { title: { ilike: "%draft%" } } }] }
```

## Patterns

`like` and `ilike` match the same way on every dialect and in memory.

```ts
const discounts: Condition<Select<typeof note>> = { title: { like: "100\\%" } };
const quarters: Condition<Select<typeof note>> = { title: { ilike: "q_ report" } };
```

## Predicates

`Condition.resolve` turns a condition into a `Predicate` that tests rows in memory.

```ts
const predicate = Condition.resolve(where, Namespace.fields(note));
Predicate.matches(Predicate.compile(predicate, note), row);
```

## Expressions

An `Expression` computes a value from one row, in memory or as SQL.

```ts
const mode = Expression.scalar(
    Expression.path(Expression.column("value"), "editor", "mode"),
    "text",
);
const converted = Expression.object({
    mode: Expression.case(mode, [{ when: "emacs", then: Expression.literal("standard") }], mode),
    isCompact: Expression.json(false),
});
Expression.evaluate(converted, row);
await database.select({ value: Expression.render(converted, setting) }).from(setting);
```

## Migrations

`plan` lists the steps from the applied tables to the declared ones, and `apply` runs them.

```ts
await database.migrate([note]);

const plan = await database.plan(mergeStates(releases.map((state) => state.tables.sqlite)));
// { steps: [{ action: "create", target: "table/note/column/priority", risk: "safe", detail: "add column priority" }] }
await database.apply(plan);
```

## Releases

`plan` compares the release that applied the tables with the releases that declare them.

```text
upgrade   applied 2026.9.0, declared 2026.10.0       -> the changes, then each conversion since 2026.9.0
rollout   declared 2026.9.0 and 2026.10.0 together   -> every column either declares, renamed columns kept in sync
rollback  applied 2026.10.0, declared 2026.9.0       -> nothing when 2026.9.0 still reads the tables, otherwise refused
```

## Log

`database.log.follow` reads the committed changes of logged tables in commit order.

```ts
const position = await database.log.position();
const selection = { tables: [note], scopes: [spaceId], after: position.sequence };
for await (const page of database.log.follow(selection, signal)) {
    apply(page.changes);
}
await database.log.advance("notes/published", consumed, Date.now() + maxLag);
await database.log.drop("notes/published");
```

### Append-only tables

`log: { appendOnly: true }` logs a table's insertions alone: updates are refused, and the deletions that prune rows once they are kept elsewhere stay unlogged, so readers following the log see each row once.

```ts
export const event = defineTable("event", columns, { log: { appendOnly: true } });
await database.update(event).set({ value: 3 }); // refused: append-only table
await database.delete(event).where(lt(event.time, sealedBefore)); // no log entries
```

## Log origins

`log.asReplica` logs each change it writes under the origin it replicates.

```ts
await transaction.log.asReplica(() => write(transaction), sourceEpoch); // changes carry origin sourceEpoch
const reached = await database.log.position(sourceEpoch); // what a follower of origin sourceEpoch still needs
```

## Log scopes

The log files each change under its row's scope.

```ts
await database.log.create(spaceId); // once, when a space's resource database is created
```

## Snapshots

`database.log.at` returns a `Snapshot` that reads the logged columns as they were at a log position.

```ts
const snapshot = database.log.at(position);
await snapshot.rows(note, { scope: spaceId });
await snapshot.ordered(note, { where, orderBy: { title: "asc" }, limit: 20 });
await snapshot.windows(comment, {
    where: {},
    orderBy: { createdAt: "desc" },
    limit: 5,
    partition: { column: "noteId", values: noteIds },
}); // the first comments of each note, in one statement
```

## Overlays

`layer` reads another layer's rows, such as a branch's, over a snapshot.

```ts
const branched = Snapshot.live(database).layer(async (table) => rowsByKey.get(table) ?? new Map());
```

## Channels

`openChannel` wakes a connection's log readers when other writers commit.

```ts
// share a SQLite file between processes on one machine, meeting in the person's runtime directory
const database = await connectBunSqlite(path, main, {
    openChannel: (name) => socketChannel(`${path}#${name}`),
});

// share a database between the tabs and workers of one origin
const openTabChannel: OpenChannel = (name) => broadcastChannel(name);

// hear PostgreSQL's commits, which the log's commit trigger announces on postgresChannel
const pool = await connectPostgres(url, main);

// open no channel for a single connection, such as a Durable Object's storage
const storage = connectDurableObject(state.storage, main);
```

## Aggregates

`aggregates` keeps counts, sums, minimums and maximums of referencing rows in the referenced table.

```ts
aggregates: [
    { into: () => folder, column: "noteCount", key: "folderId", function: "count", where: { archived: false } },
    { into: () => folder, column: "lastEditedAt", key: "folderId", function: "max", value: "editedAt" },
],
```

## Dependents

`dependents` cascades or restricts deletes to the rows that reference a row.

```ts
dependents: [{ from: () => comment, key: "parentId", where: { parentType: "note" }, onDelete: "cascade" }],
```

## Trees

`tree` keeps an ancestor index of a table's single-parent hierarchy per scope.

```ts
export const folder = defineTable("folder", columns, {
    tree: { id: "id", scope: "scope", parent: "parentId" },
});
```

## Shared databases

`serveBrowserDatabase` shares a tab's SQLite database with other tabs and workers.

```ts
const channel = broadcastChannel("notes");
const stop = await serveBrowserDatabase("notes", channel);
const database = connectShared(channel, party, main);
```

## Errors

A database failure throws a `DatabaseError` with a stable code.

```ts
import { DatabaseError } from "@destack/db";

new DatabaseError("DUPLICATE", "a record with the same unique key exists").toServiceError(); // { code: "CONFLICT", … }
```

## Tests

`TestDatabase.create` opens an isolated database per dialect.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => storage.close());
});
```
