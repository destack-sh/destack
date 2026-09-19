Interact with SQL databases in Destack.

## Usage

```ts
import { record, text } from "@destack/db";

const note = record("note", "note", {
    title: text("title").notNull(),
});
```

## Connections

Connections use Drizzle's SQLite query and transaction API.

```ts
import { connect } from "@destack/db/turso";

const database = connect("space.db");
await database.$client.connect();
await database.select().from(note);
```

## Resources

```ts
import { defineDatabase } from "@destack/db/declare";

export const main = defineDatabase({
    name: "main",
    spec: { dialect: "sqlite" },
});

const database = main.get(context);
await database.select().from(note);
await database.transaction(async (transaction) => {
    await transaction.insert(note).values({ title: "New note" });
});
```

The host binds an asynchronous SQLite connection for each invocation.

## Schemas

Define tables and migrations independently of the database resource.

```ts
import { defineDatabaseSchema } from "@destack/db/declare";
import * as tables from "./table/index.ts";

export const notes = defineDatabaseSchema({
    name: "notes",
    tables,
    migrations: new URL("./migration/", import.meta.url),
});
```

## Migrations

Apply committed Drizzle SQL before starting consumers.

```ts
import { migrate } from "@destack/db/turso";

await migrate(database, notes);
```

- Schema names remain stable and unique within a database.
- Each schema manages a distinct set of tables and one migration history.
- Migration directories use Drizzle's `YYYYMMDDHHmmss_name/migration.sql` format.
- SQL statements use Drizzle's `--> statement-breakpoint` separator.
- Applied names and checksums must match the committed history.
- Migrations and history updates run in one write transaction.
- Unmanaged tables remain unchanged unless migration SQL explicitly changes them.
- Migration SQL is trusted code and must support transactional execution.
- Cross-schema dependencies determine preparation order in the host.

## Inspection

```ts
import { describeSchema, inspectDatabase, inspectMigrations } from "@destack/db/sqlite/inspect";
import { describeMigration, readMigrations } from "@destack/db/migration";

const declaration = describeSchema(notes);
const migrations = (await readMigrations(notes)).map(describeMigration);
const snapshot = await inspectDatabase(database);
const history = await inspectMigrations(database, notes);
```
