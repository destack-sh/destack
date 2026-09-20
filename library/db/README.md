Interact with SQL databases in Destack.

## Usage

```ts
import { record, text } from "@destack/db";

const note = record("note", "note", {
    title: text("title").notNull(),
});
```

## Connections

Connections use logical tables and Drizzle query builders for the selected SQL dialect.

```ts
import { connect } from "@destack/db/turso";

const database = await connect("space.db", [note]);
await database.select().from(note);
await database.close();
```

## Columns

```ts
import { bigint, binary, integer, json, numeric, text, timestamp } from "@destack/db";

integer("count"); // safe JavaScript number
bigint("sequence"); // signed 64-bit bigint
numeric("amount"); // exact decimal string
binary("content"); // Uint8Array
timestamp("time"); // Date, UTC milliseconds
text("state", { enum: ["open", "closed"] });
json("value", valueSchema);
```

SQLite stores exact decimals as text; SQL arithmetic and ordering need explicit dialect expressions.
Bigints, bytes, and dates use runtime validators; JSON APIs need explicit serializable projections.

## Queries

```ts
const selected = database.select({ title: note.title }).from(note).as("selected");
await database.select().from(selected);

const recent = database.$with("recent").as(database.select().from(note));
await database.with(recent).select().from(recent);
await database.execute(sql`SELECT 1 AS value`);
```

## Native queries

```ts
import * as turso from "@destack/db/turso/schema";
import * as postgres from "@destack/db/postgres/schema";

const nativeNote = database.schema.table(note);
await database.native.select().from(nativeNote);
```

Native queries use Drizzle's dialect-specific API and lifecycle.

## Relations

Declare relationships once; each adapter uses Drizzle's relational query builder.

```ts
import { defineDatabaseSchema, defineRelations } from "@destack/db";

const relations = defineRelations({ note, comment }, (relation) => ({
    note: {
        comments: relation.many.comment({
            from: relation.note.id,
            to: relation.comment.noteId,
        }),
    },
}));
const notes = defineDatabaseSchema({
    name: "notes",
    tables: { note, comment },
    relations,
    migrations: new URL("./migration/", import.meta.url),
});
const database = await connect("space.db", notes);
const records = await database.query.note.findMany({ with: { comments: true } });
```

## Resources

```ts
import { defineDatabase } from "@destack/db/declare";

export const main = defineDatabase({
    name: "main",
    spec: { dialect: "sqlite" },
});

const database = main.get(context, notes);
await database.select().from(note);
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title: "New title" });
});
```

The host supplies the bound connection through the invocation context.

## Transactions

```ts
await database.transaction(async (transaction) => {
    await transaction.update(note).set({ title: "Changed" });
}, { isolationLevel: "serializable", signal: AbortSignal.timeout(5_000) });

await database.close();
```

Cancellation prevents commit and waits for submitted queries before rollback. Closing waits for
submitted portable queries and transactions, then closes the client once.

## Schemas

Define tables and migrations independently of the database resource.

```ts
import { defineDatabaseSchema } from "@destack/db";
import * as tables from "./table/index.ts";

export const notes = defineDatabaseSchema({
    name: "notes",
    tables,
    dependencies: [sharedSchema],
    migrations: new URL("./migration/", import.meta.url),
});
```

## Migrations

Apply committed Drizzle SQL before starting consumers. Histories run atomically; statements that
cannot run in a transaction are rejected by the engine.

```ts
import { migrate, prepare } from "@destack/db/migration";

await migrate(database, notes);
const ready = await prepare(database, [notes]);
```

## Generation

Use `@destack/db/migration/generate` with the matching Drizzle Kit version installed.

```ts
import { generateMigrations } from "@destack/db/migration/generate";

const result = await generateMigrations({
    module: new URL("./schema.ts", import.meta.url),
    export: "notes",
    dialect: "postgresql",
    name: "add_title",
});
```

## Inspection

```ts
import { describeSchema } from "@destack/db/inspect";
import { inspectDatabase, inspectMigrations } from "@destack/db/turso/inspect";
import { describeMigration, readMigrations } from "@destack/db/migration";

const declaration = describeSchema(notes, "sqlite");
const migrations = (await readMigrations(notes, "sqlite")).map(describeMigration);
const catalog = await inspectDatabase(database.native);
const history = await inspectMigrations(database.native, notes);
```
