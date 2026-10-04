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

`alias`, `Select`, `Insert`, `Key`, `.primaryKey()` and `execute` with a schema differ from Drizzle, and every other query builder, operator and `sql` function matches it.

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

Each column parses the values it reads with its schema, so a row written outside Destack reads as typed values or throws.

```ts
const [row] = await database.execute(
    sql`SELECT count(*) AS total FROM ${note}`,
    schema.object({ total: schema.number() }),
);
```

## Relations

`defineRelations` declares each table's relations as in Drizzle's relational queries v2, and `database.query` reads rows with them.

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

`with` includes a relation's rows, and a condition on a relation tests for a related row.

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

`relations.namespace` resolves relation names in conditions and extras, so `Condition.render` renders them for a plain `select`.

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

`defineTable` declares a table's columns, constraints, log retention and the row conversions of its releases, and the columns marked `.primaryKey()` form its key in declaration order.

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

## Text order

Text compares by its UTF-8 bytes on every dialect, so PostgreSQL text columns use the `C` collation and a column created under another collation changes to it in one safe step.

```sql
ALTER TABLE "note" ALTER COLUMN "title" TYPE text COLLATE "C"
```

## Databases

`defineDatabase` declares the tables a service owns and the tables it copies from their owning service, and `copies` tells them apart.

```ts
export const main = defineDatabase({ name: "main", tables: [note], copies: [accountTable] });

const database = main.get(context);
const unapplied = await main.check(database);
const isCopied = database.copies(accountTable);
```

## Providers

`databaseProvider.sqlite` manages a host's SQLite files, and a workload opens one through the `sqlite` connector of its declaration.

```ts
import { databaseProvider } from "@destack/db/bun";

const provider = databaseProvider.sqlite(new URL("file:///var/destack/databases/"), databaseObject);
await using connection = await main.connectors["sqlite"]?.connect(binding, main); // SQLite on Bun
```

## Connections

`connect` from `@destack/db/bun`, `@destack/db/postgres` or `@destack/db/cloudflare` opens a database on that runtime.

```ts
import { connect } from "@destack/db/bun";
import { connect as connectPostgres } from "@destack/db/postgres";
import { connect as connectStorage } from "@destack/db/cloudflare";

const file = await connect("notes.db", main);
const pool = await connectPostgres(process.env.DATABASE_URL, main);
const storage = connectStorage(state.storage, main);
```

## Sole writers

A pool that writes its database alone connects as the `sole` writer without `LISTEN`, and `postgresConnector` opens a migrated database that way by URL.

```ts
import { connect, postgresConnector } from "@destack/db/postgres";

const only = await connect(url, main, "sole");
await using placed = await postgresConnector.connect(
    { reference: env.DATABASE.connectionString },
    main,
);
```

## Writes

`transaction` runs queries in a transaction, `upsert` writes rows as they are, `remove` deletes rows by key, and `rehearse` runs a transaction and rolls it back.

```ts
const database = await connect("notes.db", main);
await database.transaction(async (transaction) =>
    transaction.update(note).set({ title: "Changed" }).where(eq(note.id, id)),
);
await database.upsert(note, rows);
await database.remove(note, keys);
const plan = await database.rehearse((transaction) => transaction.plan(state)); // rolled back
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

A `Condition` is plain JSON in the shape of Drizzle's relational filters, typed by its row, and `Condition.render` turns it into SQL.

```ts
const where: Condition<Select<typeof note>> = {
    scope: spaceId,
    OR: [{ title: { ilike: "travel%" } }, { title: { in: ["Inbox", "Ideas"] } }],
    NOT: { title: "Draft" },
};
await database.select().from(note).where(Condition.render(where, note));
```

## Patterns

`like` and `ilike` match `%` against any run of characters and `_` against one character, `\` escapes the next character, and `ilike` ignores ASCII case only, on every dialect and in memory.

```ts
const discounts: Condition<Select<typeof note>> = { title: { like: "100\\%" } };
const quarters: Condition<Select<typeof note>> = { title: { ilike: "q_ report" } };
```

## Predicates

`Condition.resolve` turns a condition into a `Predicate` against a `Namespace` of fields and relations, and `Predicate.matches` tests a row against it in memory.

```ts
const predicate = Condition.resolve(where, Namespace.fields(note));
Predicate.matches(Predicate.compile(predicate, note), row);
```

## Expressions

An `Expression` computes a value from one row, `Expression.evaluate` runs it in memory, and `Expression.render` renders it as SQL.

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

`plan` lists the steps from the applied tables to the declared ones by paths below `table/<name>`, and `apply` runs them.

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

`database.log.follow` reads the committed changes of logged tables in commit order without sensitive columns, and `advance` and `drop` manage a named reader position.

```ts
const position = await database.log.position();
const selection = { tables: [note], scopes: [spaceId], after: position.sequence };
for await (const page of database.log.follow(selection, signal)) {
    apply(page.changes);
}
await database.log.advance("notes/published", consumed, Date.now() + maxLag);
await database.log.drop("notes/published");
```

## Log scopes

The log files a change under its row's scope column, or under the scope `log.create` gives the database for a table without one.

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

`layer` puts an overlay on a snapshot that reads another layer's rows, such as a branch's, in place of the database's.

```ts
const branched = Snapshot.live(database).layer(async (table) => rowsByKey.get(table) ?? new Map());
```

## Channels

`openChannel` gives a connection a `Channel` on which other writers announce commits, which wakes its log readers as PostgreSQL's `LISTEN` and `NOTIFY` do.

```ts
// share a SQLite file between processes on one machine, meeting in the person's runtime directory
const database = await connect(path, main, {
    openChannel: (name) => socketChannel(`${path}#${name}`),
});

// share a database between the tabs and workers of one origin
const openTabChannel: OpenChannel = (name) => broadcastChannel(name);

// hear PostgreSQL's commits, which the log's commit trigger announces on postgresChannel
const pool = await connectPostgres(url, main);

// open no channel for a single connection, such as a Durable Object's storage
const storage = connectStorage(state.storage, main);
```

## Blobs

A `blob` column holds the SHA-256 digest of content in a `BlobStore`.

```ts
const attachment = defineTable("attachment", {
    id: text("id").primaryKey(),
    content: blob("content"),
});
const digest = await blobs.write(body);
await database.insert(attachment).values({ id, content: digest });
```

## Retired blobs

A `BlobStore` deletes the blobs no row references, so a writer holds it while it writes blobs and retires the blobs of the rows it deletes.

```ts
await using _held = await blobs.hold();
const digest = await blobs.write(body);
await database.insert(attachment).values({ id, content: digest });
await blobs.retire([previous]);
```

## Aggregates

`aggregates` keeps counts, sums, minimums and maximums of the referencing rows in a column of the referenced table.

```ts
aggregates: [
    { into: () => folder, column: "noteCount", key: "folderId", function: "count", where: { archived: false } },
    { into: () => folder, column: "lastEditedAt", key: "folderId", function: "max", value: "editedAt" },
],
```

## Dependents

`dependents` cascades or restricts the deletion of a row to the rows that reference it under a condition.

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

`serveBrowserDatabase` serves a browser tab's SQLite database on a channel, and `connectShared` connects to it from another tab or worker.

```ts
const channel = broadcastChannel("notes");
const stop = await serveBrowserDatabase("notes", channel);
const database = connectShared(channel, party, main);
```

## Tests

`TestDatabase.create` opens an isolated database per dialect, and runs PostgreSQL when `DESTACK_TEST_POSTGRES` is set.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => storage.close());
});
```
