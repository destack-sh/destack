# @destack/vault

Keep secrets in sealed versions and rotate them without redeploying.

## Overview

A vault keeps the secrets of a space, an account or an organization, after GCP Secret Manager.

- **Vault**: the secrets of one scope; an installation bound to it `put`s and `remove`s its own credentials.
- **Secret**: one named credential; its `currentVersion` is the version reads take.
- **Version**: one value, sealed at rest; `disable` hides it and `destroy` erases it for good.

```ts
import type { ClientOf } from "@destack/object/client";
import type vaultPackage from "@destack/vault/package";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof vaultPackage>,
    scope: string,
    vault: Identifier<"vault">,
    requestId: string,
) => {
    const name = "github";
    const secret = await client.secret.create({ scope, parentId: vault, name, requestId });
    const version = await client.version.create({
        scope,
        parentId: secret.id,
        value: { encoding: "text", value: "ghp_…" },
        requestId,
    });
    const currentVersion = version.number;
    await client.secret.update({ scope, id: secret.id, currentVersion, requestId });
    await client.version.destroy({ scope, id: version.id, requestId });
};
```

## Examples

### Declare a secret a package reads

A package declares the secrets it reads, and a secret with `generated` gets its first version at install.

```ts
import { defineSecret, defineVault } from "@destack/vault";

export const pushVault = defineVault({ name: "push" });
export const pushKey = defineSecret({
    name: "push-key",
    generated: { vault: "push", algorithm: "ES256" },
});
```

### Read a secret in a server

A server reads the version its deployment captured.

```ts
import type { Secret } from "@destack/resource";

async (secret: Secret) => {
    const { version, value } = await secret.read();
};
```

### Rotate a secret

A new version becomes current, and the old one is disabled once nothing reads it.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type vaultPackage from "@destack/vault/package";

async (
    client: ClientOf<typeof vaultPackage>,
    scope: string,
    secret: Identifier<"secret">,
    old: Identifier<"version">,
    requestId: string,
) => {
    const value = { encoding: "text", value: "ghp_…" } as const;
    const added = await client.version.create({ scope, parentId: secret, value, requestId });
    await client.secret.update({ scope, id: secret, currentVersion: added.number, requestId });
    await client.version.disable({ scope, id: old, requestId });
};
```

### Bind secrets in a stack

A stack declares its vaults and secrets, and binds an installation's declared secret to one at a version.

```ts
import { defineStack } from "@destack/stack";
import { defineVault } from "@destack/vault";
import vaultPackage from "@destack/vault/package";

const pushVault = defineVault({ name: "push" });
export const personal = defineStack({
    types: [vaultPackage],
    vaults: { push: pushVault.provision({ retention: { within: { days: 30 } } }) },
    secrets: { github: { parent: "push", name: "github" } },
});
```

### Use an operator's value as a secret

`Secret.of` turns a value an operator sets, such as an environment variable, into a secret with one version.

```ts
import { Secret } from "@destack/resource";

const stripe = Secret.of("sk_test_…");
```

## Comparison

### GCP Secret Manager

Versions, their states and the current alias follow GCP; three things differ.

- A deleted secret stays restorable for its recovery window, then its purge erases every value.
- `vault.put` destroys a consumer's earlier versions as it adds the new current one.
- Each version expires on its own `expiresAt`, where GCP expires the whole secret.
