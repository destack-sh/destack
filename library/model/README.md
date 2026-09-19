Define standard Destack records and relationships.

## Usage

```ts
import { eq } from "@destack/db";
import { connect } from "@destack/db/turso";
import type { Account } from "@destack/model/global";
import { relations, space } from "@destack/model/regional";

const database = connect("regional.db", { relations });
await database.$client.connect();

export function listSpaces(accountId: Account["id"]) {
    return database.select().from(space).where(eq(space.accountId, accountId));
}
```
