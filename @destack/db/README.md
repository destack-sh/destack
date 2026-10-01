# @destack/db

Declare, query, log and migrate SQL tables on SQLite and PostgreSQL.

## Tables

`defineTable` declares a table's columns, constraints, log retention and the row conversions of its releases.

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

## Databases

`defineDatabase` declares a database in the `global`, `regional` or `zonal` tier, holding each listed table once.
A database holds tables of its own tier and of wider tiers, whose rows it replicates from their home, and refuses tables of a narrower tier.
`connection.copies(table)` tells whether the database keeps a table's rows as copies: the tables of a wider tier than its own.

```ts
export const main = defineDatabase({ name: "main", tables: [note] });
export const global = defineDatabase({
    name: "global",
    tier: "global",
    tables: [...accountTables, ...hostTables],
});

const database = main.get(context);
const unapplied = await main.check(database);
```

A host manages SQLite files through `sqliteProvider`, and a workload opens them through its declaration's connectors.

```ts
import { sqliteProvider } from "@destack/db/sqlite";

const provider = sqliteProvider(new URL("file:///var/destack/databases/"));
const connection = await main.connectors.sqlite!.connect(binding, main); // SQLite on Bun, none elsewhere yet
```

## Connections

Each entry point opens one kind of database.

| Entry point | Opens |
|---|---|
| `@destack/db/bun` | `connect`: a SQLite file or memory database on Bun |
| `@destack/db/postgres` | `connect`: a PostgreSQL pool or URL |
| `@destack/db/wasm` | `serveBrowserDatabase`: an OPFS SQLite database served on a channel |
| `@destack/db/shared` | `connectShared`: a database another party serves |
| `@destack/db/channel` | `Channel`, `broadcastChannel`: messages between the parties of one topic |
| `@destack/db/channel/socket` | `socketChannel`: the processes sharing a SQLite file on one machine |
| `@destack/db/blob` | `BlobStore`, `DatabaseHandle`: the content blob columns reference |
| `@destack/db/blob/local` | `LocalBlobStore`: blobs as files named by digest |
| `@destack/db/test` | `TestDatabase`: an isolated database per dialect |

## Writes

A connection runs Drizzle queries in transactions and writes whole rows by key.

```ts
const database = await connect("notes.db", main);
await database.transaction(async (transaction) =>
    transaction.update(note).set({ title: "Changed" }).where(eq(note.id, id)),
);
await database.upsert(note, rows);
await database.remove(note, keys);
```

## Statements

A `Statement` renders once per database and runs with named values.

```ts
const notes = new Statement(
    (value) => sql`SELECT ${note.id} AS id FROM ${jsonElements(value("ids"), "listed")}
    JOIN ${note} ON ${note.id} = listed.value ->> 0`,
);
await notes.all(database, { ids: JSON.stringify(ids.map((id) => [id])) });
```

## Conditions

A `Condition` from `@destack/db/query` renders to SQL and matches rows in memory alike.

```ts
const where = Condition.all(Condition.eq("scope", spaceId), Condition.gte("rank", 2));
await database
    .select()
    .from(note)
    .where(Condition.render(where, Condition.bind(note)));
Condition.matches(Condition.compile(where, note), row);
```

## Expressions

An `Expression` computes a value from one row alike in SQL and memory, JSON included.

```ts
const mode = Expression.scalar(
    Expression.path(Expression.column("value"), "editor", "mode"),
    "text",
);
const converted = Expression.object({
    mode: Expression.case(mode, [{ when: "emacs", then: Expression.literal("standard") }], mode),
    pinned: Expression.json(false),
});
Expression.evaluate(converted, row);
await database.select({ value: Expression.render(converted, setting) }).from(setting);
```

## Migrations

A connection plans the steps from its applied tables to declared ones, addressed `table/<name>` and its parts.

```ts
await database.migrate([note]);

const plan = await database.plan(mergeStates(releases.map((state) => state.tables.sqlite)));
// { steps: [{ action: "create", target: "table/note/column/priority", risk: "safe", detail: "add column priority" }] }
await database.apply(plan);
```

A plan compares the release that applied the tables with the releases declaring them.

| Case | Example | Plan |
|---|---|---|
| Upgrade | applied 2026.9.0, declared 2026.10.0 | the changes, then each conversion since 2026.9.0 |
| Rollout | declared 2026.9.0 and 2026.10.0 together | every column either declares, renamed columns kept in sync |
| Rollback | applied 2026.10.0, declared 2026.9.0 | nothing when 2026.9.0 still reads the tables, otherwise refused |

## Log

`database.log` reads committed changes of logged tables in commit order.

```ts
const position = await database.log.position();
const selection = { tables: [note], scopes: [spaceId], after: position.sequence };
for await (const page of database.log.follow(selection, signal)) {
    apply(page.changes);
}
await database.log.advance("notes/published", consumed, Date.now() + maxLag);
await database.log.drop("notes/published");
```

A change files under its row's scope column, or under the database's scope for a table without one.

```ts
await database.log.create(spaceId); // once, when a space's resource database is created
```

## Snapshots

A `Snapshot` reads the logged columns as they were at a log position.

```ts
const snapshot = database.log.at(position);
await snapshot.rows(note, Condition.eq("scope", spaceId));
await snapshot.ordered(note, { where, order: [{ column: "title", direction: "asc" }], count: 20 });
```

A snapshot under an `Overlay` reads another layer's rows in place of the database's, such as a branch's.

```ts
const branched = Snapshot.live(database).layer(async (table) => rowsByKey.get(table) ?? new Map());
```

## Channels

A connection wakes its log readers on the commits other writers announce on its `Channel`, as PostgreSQL's `LISTEN` and `NOTIFY` do.

| Channel | Writers |
|---|---|
| none | one connection, such as a Durable Object's storage |
| `broadcastChannel(name)` | tabs and workers of one origin |
| `socketChannel(path)` | processes on one machine sharing a SQLite file |
| `postgresChannel(client, name)` | PostgreSQL, announced by the log's commit trigger |

```ts
const database = await connect(path, main, { channel: socketChannel(path) });
```

## Blobs

A `blob` column keeps the SHA-256 digest of content a `BlobStore` keeps.

```ts
const attachment = defineTable("attachment", { id: text("id").primaryKey(), content: blob("content") });
const stored = await blobs.write(body);
await database.insert(attachment).values({ id, content: stored.digest });
```

## Aggregates

A table keeps counts, sums and extremes of the rows referencing it.

```ts
aggregates: [
    { into: () => folder, column: "noteCount", key: "folderId", function: "count", where: { archived: false } },
    { into: () => folder, column: "lastEditedAt", key: "folderId", function: "max", value: "editedAt" },
],
```

## Dependents

A table cascades or restricts its deletion to rows referencing it under a condition.

```ts
dependents: [{ from: () => comment, key: "parentId", where: { parentType: "note" }, onDelete: "cascade" }],
```

## Trees

A table with a `tree` option keeps an ancestor index of its single-parent hierarchy per scope.

```ts
export const folder = defineTable("folder", columns, {
    tree: { id: "id", scope: "scope", parent: "parentId" },
});
```

## Shared databases

A browser tab serves its SQLite database to other parties on a channel.

```ts
const channel = broadcastChannel("notes");
const stop = await serveBrowserDatabase("notes", channel);
const database = connectShared(channel, party, main);
```

## Tests

`TestDatabase` opens an isolated database per dialect, PostgreSQL when `DESTACK_TEST_POSTGRES` is set.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => test.close());
});
```
