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
            "2026.10.0": { title: Expression.coalesce(Expression.column("title"), Expression.literal("")) },
        },
    },
);
```

## Databases

`defineDatabase` declares a database in the `global`, `regional` or `zonal` tier, holding each listed table once and refusing tables declared for another tier.

```ts
export const main = defineDatabase({ name: "main", tables: [note] });
export const global = defineDatabase({ name: "global", tier: "global", tables: [...accountTables, ...hostTables] });

const database = main.get(context);
const unapplied = await main.check(database);
```

A database declaration opens its kind's providers on the running runtime: SQLite on Bun, none elsewhere yet.

```ts
const provider = await main.providers.sqlite!(new URL(reference));
```

## Connections

Each entry point opens one kind of database.

| Entry point | Opens |
|---|---|
| `@destack/db/bun` | `connect`: a SQLite file or memory database on Bun |
| `@destack/db/postgres` | `connect`: a PostgreSQL pool or URL |
| `@destack/db/wasm` | `serveBrowserDatabase`: an OPFS SQLite database served to a relay |
| `@destack/db/shared` | `connectShared`: a database another party serves |
| `@destack/db/relay` | `broadcastRelay`: the channel between parties and the owner |
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
const notes = new Statement((value) => sql`SELECT ${note.id} AS id FROM ${jsonElements(value("ids"), "listed")}
    JOIN ${note} ON ${note.id} = listed.value ->> 0`);
await notes.all(database, { ids: JSON.stringify(ids.map((id) => [id])) });
```

## Conditions

A `Condition` from `@destack/db/query` renders to SQL and matches rows in memory alike.

```ts
const where = Condition.all(Condition.eq("scope", spaceId), Condition.gte("rank", 2));
await database.select().from(note).where(Condition.render(where, Condition.bind(note)));
Condition.matches(Condition.compile(where, note), row);
```

## Expressions

An `Expression` computes a value from one row alike in SQL and memory, JSON included.

```ts
const mode = Expression.scalar(Expression.path(Expression.column("value"), "editor", "mode"), "text");
const converted = Expression.object({
    mode: Expression.case(mode, [{ when: "emacs", then: Expression.literal("standard") }], mode),
    pinned: Expression.json(false),
});
Expression.evaluate(converted, row);
await database.select({ value: Expression.render(converted, setting) }).from(setting);
```

## Migrations

A connection plans and applies the changes from its tables to the declared ones, where column values narrow only by a release's conversion.

```ts
await database.migrate([note]);

const plan = await database.plan(desiredStates);
await database.apply(plan);
```

## Log

`database.log` reads committed changes of logged tables in commit order.

```ts
const position = await database.log.position();
const selection = { tables: [note], scopes: [spaceId], after: position.sequence };
for await (const page of database.log.follow(selection, signal)) {
    apply(page.changes);
}
await database.log.hold("notes/published", consumed, Date.now() + maxLag);
await database.log.release("notes/published");
```

## Snapshots

A `Snapshot` reads the logged columns as they were at a log position.

```ts
const snapshot = database.log.at(position);
await snapshot.rows(note, Condition.eq("scope", spaceId));
await snapshot.ordered(note, { where, order: [{ column: "title", direction: "asc" }], count: 20 });
```

## Commit notifiers

A `CommitNotifier` wakes log readers when another writer commits.

| Notifier | Use |
|---|---|
| `soleWriter` | A database with one writing process, the SQLite default |
| `relayNotifier(relay)` | Writers sharing a relay |
| PostgreSQL | Built in, through `LISTEN` and `pg_notify` |

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

## Replication

`Replication` copies a database's tables into another database as chunks, exactly across dialects.

```ts
const replication = Replication.of(desiredStates, source.dialect);
for await (const chunk of replication.export(source, name, "live", cursor, signal)) {
    await replication.import(target, chunk);
}
```

## Shared databases

A browser tab serves its SQLite database to other parties over a relay.

```ts
const relay = broadcastRelay("notes");
const stop = await serveBrowserDatabase("notes", relay);
const database = connectShared(relay, party, main);
```

## Tests

`TestDatabase` opens an isolated database per dialect, PostgreSQL when `DESTACK_TEST_POSTGRES` is set.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => test.close());
});
```
