Declare vaults and reference secrets in Destack.

## Usage

```ts
import { defineSecret, defineVault } from "@destack/vault";

export const credentials = defineVault({
    name: "credentials",
    spec: {},
});

export const mailToken = defineSecret({ name: "mail-token" });
```

## Bindings

```ts
import { SecretReference } from "@destack/vault";

const current = SecretReference.parse({ space: space.id, secret: token.id });
const pinned = SecretReference.parse({ space: space.id, secret: token.id, version: 3 });
```
