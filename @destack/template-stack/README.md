Declare the base configuration of a space.

```ts
// src/settings/index.ts
export { appearance } from "@destack/theme/settings";
```

```ts
// src/stack/settings.ts
import { defineSettingAssignment } from "@destack/setting/declare";
import { appearance } from "../settings/index.ts";

export const personalAppearance = defineSettingAssignment(
    appearance,
    {
        kind: "user",
        user: {
            kind: "user",
            authority: "global",
            id: "user-019f5530-8000-7000-8000-000000000003",
        },
    },
    "dark",
);
```

```ts
import { defineAccount } from "@destack/model/declare";

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
