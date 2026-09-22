Define standard Destack records and relationships.

## Declarations

```ts
import { defineAccount } from "@destack/model/declare";

export const account = defineAccount({
    environments: {
        development: {},
        production: { tags: { purpose: "live" } },
    },
});
```

## Usage

```ts
import { eq } from "@destack/db";
import { connect } from "@destack/db/turso";
import type { Account } from "@destack/model/global";
import { prepare } from "@destack/db/migration";
import { globalSchema, space } from "@destack/model/global";

const connection = await connect("global.db", globalSchema);
const database = await prepare(connection, [globalSchema]);

export function listSpaces(accountId: Account["id"]) {
    return database.select().from(space).where(eq(space.accountId, accountId));
}
```
