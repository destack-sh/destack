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
await database.select().from(note);
```

## Inspection

```ts
import { inspectDatabase } from "@destack/db/sqlite/inspect";

const snapshot = await inspectDatabase(database);
```
