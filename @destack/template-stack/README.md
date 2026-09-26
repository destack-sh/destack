Declare the base configuration of a space.

```ts
// src/settings/index.ts
export { appearance } from "@destack/theme/settings";
```

```ts
// src/space/space.ts
import { defineSpace } from "@destack/space";
import { appearance } from "../settings/index.ts";

export const personal = defineSpace({
    settings: {
        appearance: { setting: appearance, value: "dark", mode: "recommend" },
    },
});
```

```ts
import { defineAccount } from "@destack/account/declare";

export const account = defineAccount({
    environments: { development: {}, production: {} },
});
```

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
