Declare SQL tables and databases, query them on SQLite and PostgreSQL, and migrate them by plan.

## Tables

A table declares its columns, constraints, log, tree, aggregates, version, previous names and conversions.

```ts
export const note = defineTable(
    "note",
    {
        id: identifier("id", "note").primaryKey(),
        spaceId: identifier("space_id", "space").notNull(),
        title: text("title").notNull(),
    },
    {
        constraints: (note) => [index("note_space").on(note.spaceId)],
        log: { tier: "history", route: "spaceId" },
        moved: { columns: { title: "name" } },
        version: 2,
        convert: { 2: (note) => ({ title: sql`trim(${note.title})` }) },
    },
);
```

## Databases

A database names its tables, supports every dialect, and a resource context connects it.

```ts
export const main = defineDatabase({ name: "main", tables: [note] });

const database = main.get(context);
await database.transaction(async (transaction) => transaction.update(note).set({ title: "Changed" }));
const unapplied = await main.check(database);
```

## Connections

Each entry point opens one kind of database.

| Entry point | Opens |
|---|---|
| `@destack/db/turso` | A SQLite file or memory database, as a `SqliteDatabase` |
| `@destack/db/turso/serverless` | A hosted Turso database |
| `@destack/db/postgres` | A PostgreSQL server, as a `PostgresDatabase` |
| `@destack/db/wasm` | A browser SQLite database through a `WasmClient` |
| `@destack/db/shared` | A database another party owns, over a `Channel` |
| `@destack/db/channel` | `Channel` and `broadcastChannel`, the message transport of shared databases and `channelNotifier` |
| `@destack/db/sqlite` | `sqliteProvider(root)`: SQLite files as provisioned resources |

## Plans

A plan takes the states recorded in `__destack_state` to the declared states.

```ts
const plan = await planMigration(database, declareState([note], "sqlite"));
await applyPlan(database, plan);

await migrate(database, [note]);
await migrate(replica, [note], { isReplica: true });
```

| Step risk | Example |
|---|---|
| `safe` | Add a nullable column |
| `data-dependent` | Add a unique index |
| `backward-incompatible` | Rename a table or column |
| `destructive` | Drop a table |

## Log

Triggers record every committed change of a logged table, and readers follow it by `LogPosition`.

```ts
const position = await database.log.position();
for await (const page of database.log.follow({ tables: [note], after: position.sequence }, signal)) {
    apply(page.changes);
}
await database.log.wait(sequence, signal);
await database.log.renew();
await database.transaction(async (transaction) => transaction.log.copying(() => copy(transaction)));
```

Readers wake through the connection's `CommitWatch`, fed by its `CommitNotifier`.

| Notifier | Listens | Notifies |
|---|---|---|
| PostgreSQL | `LISTEN` on the change channel | `pg_notify` in the change trigger |
| `channelNotifier(channel)` | Commit messages | Posts a commit message |
| `pollNotifier(interval)` | The latest sequence | The next poll |

## Aggregates

A child table declares the aggregates its parents hold, and triggers keep them current.

```ts
aggregates: [
    { into: () => folder, column: "noteCount", key: "folderId", function: "count", where: { archived: false } },
    { into: () => folder, column: "lastEditedAt", key: "folderId", function: "max", value: "editedAt" },
],
```

## Shared databases

Parties reach a database through the one owner that serves a `Channel`.

```ts
const stop = await serveBrowserDatabase("notes", channel);
const database = connectShared(channel, origin, tables);
```

| Message | From | Meaning |
|---|---|---|
| `join` | Party | Ask which owner serves |
| `serving` | Owner | Name the owner answering from now on |
| `request` | Party | Run a step on the named owner |
| `answer` | Owner | Settle one request |
| `commit` | Any | Wake readers |

## Tests

`TestDatabase` opens an isolated database per dialect in `TEST_DIALECTS`, with PostgreSQL when `DESTACK_TEST_POSTGRES` names a server.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => test.close());
});
```
