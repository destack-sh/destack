Interact with SQL databases in Destack.

## Usage

```ts
import { describeDatabase, record, text } from "@destack/db";

const note = record("note", "note", {
    title: text("title").notNull(),
});

const description = describeDatabase("main", { note });
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

Schema inspection associates tables with the database declaration's name.
The host binds an asynchronous SQLite connection for each invocation.

## Inspection

```ts
import { inspectDatabase } from "@destack/db/sqlite/inspect";

const snapshot = await inspectDatabase(database);
```
