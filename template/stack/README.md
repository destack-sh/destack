Declare the base configuration of a space.

## Usage

Set the copied package name to your account's stack package, such as `@florian/stack`.

```ts
import { credentials, database, files } from "@florian/stack";
```

## Policies

Edit `src/policy/policy.ts` to change package admission or outbound connections.

```ts
import { defineSpace } from "@destack/space";
import { packages, network } from "@florian/stack";

export const personal = defineSpace({
    policies: { packages, network },
});
```
