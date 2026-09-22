Define standard Destack records and relationships.

## Usage

```ts
import { eq } from "@destack/db";
import { connect } from "@destack/db/turso";
import type { Account } from "@destack/model/global";
import { prepare } from "@destack/db/migration";
import { regionalSchema, space } from "@destack/model/regional";

const connection = await connect("regional.db", regionalSchema);
const database = await prepare(connection, [regionalSchema]);

export function listSpaces(accountId: Account["id"]) {
    return database.select().from(space).where(eq(space.accountId, accountId));
}
```
