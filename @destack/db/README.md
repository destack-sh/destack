# @destack/db

`@destack/db` runs Drizzle's query builder on SQLite and PostgreSQL, with drivers for Bun, Postgres.js, Durable Objects and the browser (`/browser`), and `Filter` reads Google's AIP-160 filters.

```ts
// Drizzle's select
const notes = await database.select().from(note).where(eq(note.scope, spaceId)).limit(50);
// Drizzle's insert
await database.insert(note).values(row).onConflictDoUpdate({ target: note.id, set: { title } });
// Drizzle's relational queries v2
await database.query.project.findMany({ with: { tasks: { limit: 3 } } });
type NoteRow = Select<typeof note>; // Drizzle's $inferSelect
// Drizzle's relational filters as JSON
await database
    .select()
    .from(note)
    .where(Condition.render({ title: { ilike: "q_%" } }, note));
Filter.parse('status = open AND -title:"draft"', { fields, values }); // AIP-160
await database.transaction(async (transaction) => transaction.update(note).set({ title }));
await database.execute(sql`SELECT count(*) AS count FROM ${note}`, Counted); // rows parsed
await database.run(sql`DELETE FROM ${note} WHERE ${note.scope} = ${spaceId}`); // no rows
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
        constraints: (note) => [
            index("note_scope").on(note.scope),
            check("note_title", { title: { length: { lte: 200 }, NOT: { startsWith: " " } } }),
        ],
        log: { retention: "history" },
        convert: {
            "2026.10.0": {
                title: Expression.coalesce(Expression.column("title"), Expression.literal("")),
            },
        },
    },
);
```

## Predefined rows

A table's `predefined` rows are the same in every database, and writes to them fail.

```ts
export const role = defineTable("role", columns, {
    log: {},
    predefined: [{ id: "role-owner", scope: "universe", name: "owner" }],
});
await Transaction.run(database, (transaction) =>
    transaction.write({
        table: role,
        before: null,
        after: { id: "role-owner", scope: "universe", name: "admin" },
    }),
); // INVALID_RECORD: cannot write predefined row ["role","role-owner"] of role
```

## Full-text search

A table's `fullText` columns keep a search index, and `SEARCH` matches rows that contain each word as a prefix.

```ts
export const note = defineTable("note", columns, { log: {}, fullText: ["title", "body"] });
await database
    .select()
    .from(note)
    .where(Condition.render({ SEARCH: "lov ada" }, note));
note[TABLE].requireFullText().matches(row, SearchText.query("lov ada")); // the same match in memory
```

## Databases

`defineDatabase` declares the tables a package owns and replicates, and `defineRelations` relates them, as Drizzle's does.

```ts
export const relations = defineRelations({ project, task }, (r) => ({
    project: { tasks: r.many.task({ from: r.project.id, to: r.task.projectId }) },
    task: { project: r.one.project({ from: r.task.projectId, to: r.project.id }) },
}));
export const work = defineDatabase({ name: "work", tables: [project, task], relations });

const database = work.get(context);
```

## Connections

`connectBunSqlite` and `connectPostgres` open a database on Bun and PostgreSQL, and `DurableObjectDatabaseHost` opens one in a Durable Object.

```ts
const file = await connectBunSqlite("work.db", work);
const pool = await connectPostgres(url, work);
const storage = new DurableObjectDatabaseHost(state.storage).connect(work, { namespace: "work" });
```

## Sealed columns

`sensitive()` columns are encrypted at rest under a key derived from the database's scope, and `sensitive("deterministic")` columns can still be looked up.

```ts
const credential = defineTable("credential", {
    id: text("id").primaryKey(),
    secret: text("secret").notNull().sensitive(),
    token: text("token").notNull().sensitive("deterministic"),
});
database.key(await ColumnCipher.derive(spaceIdentity));
await database.select().from(credential).where(eq(credential.token, token));
```

## Migrations

`plan` lists the steps from the applied tables to the declared ones, and `apply` runs them.

```ts
const plan = await database.plan(mergeStates([previous.tables.sqlite, next.tables.sqlite]));
// { steps: [{ action: "create", target: "table/note/column/priority", risk: "safe", … }] }
await database.apply(plan);
```

## Transactions

`Transaction.open` reads at a snapshot, buffers its writes, and commits only when nothing it read changed.

```ts
const transaction = await Transaction.open(database);
const before = present(await transaction.read(note).findFirst({ where: { id } }), "note");
const recent = await transaction
    .read(note)
    .findMany({ where: { scope }, orderBy: { id: "desc" }, limit: 10 });
await transaction.write(
    { table: note, before, after: { ...before, title: "Plans" } },
    { table: note, before: null, after: created },
);
await transaction.commit();
```

## Shared reads

A table declared `log: { isShared: true }` lets transactions share their reads of it, each read kept at its tables' latest change until a commit writes them.

```ts
export const member = defineTable("member", columns, { log: { isShared: true } });
const transaction = await Transaction.open(database); // reads shared tables through database.sharing()
```

## Log

`database.log.follow` reads the committed changes of logged tables in order, and `database.log.at` reads the tables as of a position.

```ts
const position = await database.log.position();
for await (const page of database.log.follow(
    { tables: [note], scopes: [spaceId], after: position.sequence },
    signal,
)) {
    apply(page.changes);
}
await database.log
    .at(position)
    .read(note)
    .findMany({ where: { scope: spaceId } });
```

## Leases and rate limits

`Lease` fences a key's old holders by epoch, as Kubernetes leader election does, and `Bucket` takes rate tokens by the generic cell rate algorithm.

```ts
const taken = await Lease.acquire(database, {
    name: "controller:sync",
    key,
    holder,
    duration: 15_000,
    now,
});
await Lease.release(database, { holder, name: "controller:sync" });
const outcome = await Bucket.take(database, {
    name: "method:notes/post.create",
    key: spaceId,
    rate: { count: 2, every: 60_000 },
    burst: 2,
    now,
}); // { kind: "taken" } | { kind: "limited", retryAfter }
```

## Derived columns

`aggregates` keeps counts, sums, minimums and maximums of referencing rows, `dependents` cascades deletes, and `tree` keeps an ancestor index.

```ts
export const folder = defineTable("folder", columns, {
    tree: { id: "id", scope: "scope", parent: "parentId" },
});
export const note = defineTable("note", columns, {
    aggregates: [
        { into: () => folder, column: "noteCount", key: "folderId", function: "count", where: {} },
    ],
    dependents: [{ from: () => comment, key: "noteId", where: {}, onDelete: "cascade" }],
});
```

## Tests

`TestDatabase.create` opens an isolated database per dialect.

```ts
test.for(TEST_DIALECTS)("keep notes on %s", async (dialect) => {
    const storage = await TestDatabase.create(dialect, [note]);
    onTestFinished(() => storage.close());
});
```

## Templates

A `TestTemplate` is the rows tests start with, written once per run and copied into each test by `TestDatabase.copy`.

```ts
export const templates = [
    new TestTemplate({
        name: "notes.fixture",
        tables: [note],
        value: schema.null(),
        write: keepFixture,
    }),
];
export default TestRun.setup(new URL("./templates.ts", import.meta.url)); // src/test/setup.ts, the config's globalSetup
const { storage, value } = await TestDatabase.copy(dialect, fixture);
```
