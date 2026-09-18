Define standard Destack records and relationships.

## Usage

```ts
import { eq } from "@destack/db";
import { connect } from "@destack/db/turso";
import type { Account } from "@destack/model/account";
import { space } from "@destack/model/space";

const database = connect("system.db");
export function listSpaces(accountId: Account["id"]) {
    return database.select().from(space).where(eq(space.accountId, accountId));
}
```

## Relations

```ts
import { accountRelations } from "@destack/model/account";
import { spaceRelations } from "@destack/model/space";
import { connect } from "@destack/db/turso";

const database = connect("system.db", {
    relations: { ...accountRelations, ...spaceRelations },
});
const accounts = await database.query.account.findMany({ with: { spaces: true } });
```
