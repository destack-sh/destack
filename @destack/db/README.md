Declare, query, log and migrate SQL tables on SQLite and PostgreSQL.

## Tables

A logged table files each change under its `scope` column.

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
        log: { tier: "history" },
        moved: { columns: { title: "name" } },
        version: 2,
        convert: { 2: (note) => ({ title: sql`trim(${note.title})` }) },
    },
);
```

## Databases

A database names its tables.

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
| `@destack/db/turso` | An embedded SQLite file or memory database |
| `@destack/db/turso/serverless` | A hosted Turso database |
| `@destack/db/postgres` | A PostgreSQL server |
| `@destack/db/wasm` | A browser SQLite database |
| `@destack/db/shared` | A database another party owns |
| `@destack/db/relay` | The transport of shared databases and commit notifications |
| `@destack/db/sqlite` | SQLite files as provisioned resources |

## Statements

A `Statement` renders once per database and runs with named values.

```ts
const notes = new Statement((value) => sql`SELECT ${note.id} AS id FROM ${jsonElements(value("ids"), "listed")}
    JOIN ${note} ON ${note.id} = listed.value ->> 0`);
await notes.all(database, { ids: JSON.stringify(ids.map((id) => [id])) });
```

A connection writes and deletes whole rows by key.

```ts
await database.upsert(note, rows); // inserts new keys, updates held ones, firing the log's triggers
await database.remove(note, keys); // deletes by key
```

## Conditions

`@destack/db/query` holds conditions and expressions that SQL and memory decide alike.

```ts
const where = Condition.all(Condition.eq("scope", spaceId), Condition.gte("rank", 2));
await database.select().from(note).where(Condition.render(where, Condition.bind(note)));
Condition.matches(Condition.compile(where, note), row);
```

| Noun | Verbs |
|---|---|
| `Condition` | `eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `oneOf`, `missing`, `all`, `any`, `not`, `exists`, `render`, `compile`, `matches`, `columns`, `require` |
| `Expression` | `column`, `literal`, `add`, `subtract`, `multiply`, `divide`, `coalesce`, `lookup`, `rollup`, `render`, `kind`, `require` |
| `Order` | `complete`, `render`, `after`, `rows` |
| `Key` | `match`, `any`, `name`, `parse` |
| `Row` | `encodeRow`, `decodeRow` |

## Plans

The connection plans and applies migrations.

```ts
await database.migrate([note]);
await replica.migrate([note], { isReplica: true });

const plan = await database.plan(desiredStates);
await database.apply(plan);
```

| Step risk | Example |
|---|---|
| `safe` | Add a nullable column |
| `data-dependent` | Add a unique index |
| `backward-incompatible` | Rename a table or column |
| `destructive` | Drop a table |

## Log

Readers follow committed changes by `LogPosition`.

```ts
const position = await database.log.position();
for await (const page of database.log.follow({ tables: [note], scopes: [spaceId], after: position.sequence }, signal)) {
    apply(page.changes);
}
await database.log.wait(sequence, signal);
await database.log.renew();
await database.transaction(async (transaction) => transaction.log.copying(() => copy(transaction)));
```

A `Snapshot` reads the database as it was at a position.

```ts
const snapshot = database.log.at(position);
await snapshot.rows(note, Condition.eq("scope", spaceId));
await snapshot.ordered(note, { where, order: [{ column: "title", direction: "asc" }], count: 20 });
```

A snapshot reads only what the log can restore.

| Rule | Reason |
|---|---|
| changed rows read as their image in the log | reads stay on current indexes |
| only logged columns exist | the log holds no images of the others |
| a position before the horizon of a windowed table fails with `CHANGES_COMPACTED` | its changes are gone |

A transaction's bounds name the positions and times around it, and a consumer's hold keeps the changes after its position until the hold expires.

```ts
const { before, after, startedAt, committedAt } = await database.log.bounds(sequence);
await database.log.hold("notes/published", consumed, Date.now() + maxLag);
await database.log.release("notes/published");
```

Readers wake on commits through a `CommitNotifier`.

| Notifier | Listens | Notifies |
|---|---|---|
| PostgreSQL | `LISTEN` on the change channel | `pg_notify` in the change trigger |
| `relayNotifier(relay)` | Commit messages | Posts a commit message |
| `pollNotifier(interval)` | The latest sequence | The next poll |

## Aggregates

A child table declares the aggregates its parents hold with `into`, or a parent declares them with `from` when the children do not know it.

```ts
aggregates: [
    { into: () => folder, column: "noteCount", key: "folderId", function: "count", where: { archived: false } },
    { into: () => folder, column: "lastEditedAt", key: "folderId", function: "max", value: "editedAt" },
],

// on the parent, counting only the rows naming its type
aggregates: [{ from: () => comment, column: "commentCount", key: "parentId", function: "count", where: { parentType: "note" } }],
```

## Dependents

A table declares the rows of another table referencing it under a condition, which delete with it or keep it from deletion with `BROKEN_REFERENCE`, as a polymorphic reference needs.

```ts
dependents: [{ from: () => comment, key: "parentId", where: { parentType: "note" }, onDelete: "cascade" }],
```

## Shared databases

Parties reach a database through the owner serving a `Channel`.

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

`TestDatabase` opens an isolated database per dialect, PostgreSQL when `DESTACK_TEST_POSTGRES` is set.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const test = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => test.close());
});
```
