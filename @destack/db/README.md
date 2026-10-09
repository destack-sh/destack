# @destack/db

`@destack/db` is Drizzle's query builder and relational queries v2 on SQLite and PostgreSQL, `/bun`, `/postgres` and `/cloudflare` are its drivers for Bun SQLite, Postgres.js and Durable Object storage, `/browser` is SQLite Wasm shared between tabs, and `Filter` reads Google's AIP-160 filters.

```ts
const notes = await database.select().from(note).where(eq(note.scope, spaceId)).limit(50); // Drizzle's select
await database.insert(note).values(row).onConflictDoUpdate({ target: note.id, set: { title } }); // Drizzle's insert
await database.query.project.findMany({ with: { tasks: { limit: 3 } } }); // Drizzle's relational queries v2
type NoteRow = Select<typeof note>; // Drizzle's $inferSelect
await database.select().from(note).where(Condition.render({ title: { ilike: "q_%" } }, note)); // Drizzle's relational filters as JSON
Filter.parse('status = open AND -title:"draft"', { fields, values }); // AIP-160
await database.transaction(async (transaction) => transaction.update(note).set({ title }));
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

## Databases

`defineDatabase` declares the tables a service owns and the tables it copies, and `defineRelations` relates them as Drizzle's `defineRelations` does.

```ts
export const relations = defineRelations({ project, task }, (r) => ({
    project: { tasks: r.many.task({ from: r.project.id, to: r.task.projectId }) },
    task: { project: r.one.project({ from: r.task.projectId, to: r.project.id }) },
}));
export const work = defineDatabase({ name: "work", tables: [project, task], relations });

const database = work.get(context);
```

## Connections

`connectBunSqlite`, `connectPostgres` and `connectDurableObject` open a database on Bun, PostgreSQL and a Durable Object's storage.

```ts
const file = await connectBunSqlite("work.db", work);
const pool = await connectPostgres(url, work);
const storage = connectDurableObject(state.storage, work, { namespace: "work" });
```

## Migrations

`plan` lists the steps from the applied tables to the declared ones, and `apply` runs them.

```ts
const plan = await database.plan(mergeStates([previous.tables.sqlite, next.tables.sqlite]));
// { steps: [{ action: "create", target: "table/note/column/priority", risk: "safe", detail: "add column priority" }] }
await database.apply(plan);
```

## Log

`database.log.follow` reads the committed changes of logged tables in commit order, and `database.log.at` reads them as they were at a position.

```ts
const position = await database.log.position();
for await (const page of database.log.follow({ tables: [note], scopes: [spaceId], after: position.sequence }, signal)) {
    apply(page.changes);
}
await database.log.at(position).rows(note, { scope: spaceId });
```

## Derived columns

`aggregates` keeps counts, sums, minimums and maximums of referencing rows, `dependents` cascades deletes, and `tree` keeps an ancestor index.

```ts
export const folder = defineTable("folder", columns, {
    tree: { id: "id", scope: "scope", parent: "parentId" },
});
export const note = defineTable("note", columns, {
    aggregates: [{ into: () => folder, column: "noteCount", key: "folderId", function: "count", where: {} }],
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
