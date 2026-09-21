Declare the base configuration of a space.

```ts
import { credentials, database, files } from "@florian/stack";
```

```ts
import { defineSpace } from "@destack/space";
import { packages, network } from "@florian/stack";

export const personal = defineSpace({
    policies: { packages, network },
});
```
