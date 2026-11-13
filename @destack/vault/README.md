# @destack/vault

Keep secrets as encrypted versions and rotate them without redeploying.

## Overview

`@destack/vault` is a secret manager after GCP Secret Manager: vaults, secrets, and numbered versions encrypted at rest.

Your package declares the vaults and secrets it uses:

- `defineVault`: declares a vault resource.
- `defineSecret`: declares a secret, generated at install or bound by a stack.
- `Secret`: a secret at the version a deployment captured.

Vault objects:

- `vault`: the secrets of one scope.
- `secret`: one named credential.
- `version`: one numbered, immutable value of a secret.
- `VaultError`: the reasons a read or write fails, checked with `VaultError.is`.

Declare a vault and a generated signing key, then read the key in your package's server:

```ts
import { t } from "@destack/locale";
import { definePackage, defineMount } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { SecretReading } from "@destack/resource";
import { defineSecret, defineVault } from "@destack/vault";

// declare a vault and a signing key generated at install
const keys = defineVault({ name: "keys" });
const signingKey = defineSecret({
    name: "signing-key",
    generated: { vault: "keys", algorithm: "ES256" },
});
const status = defineMount({ name: "status", path: "/status" });
const push = definePackage({
    title: t`Push`,
    description: t`Push notifications to devices.`,
    icon: "bell-ringing",
    color: "pink",
    resources: [keys, signingKey],
    mounts: [status],
});

export default implementPackage(push, (context) => {
    // read the bound secret
    const secret = context.resources.get(signingKey);

    return {
        mounts: {
            status: async () => {
                // read the key as a JSON Web Key
                const reading = await secret.read();
                const key = JSON.parse(SecretReading.text(reading));

                return Response.json({ version: reading.version, algorithm: key.alg });
            },
        },
    };
});
```

## Reference

### defineVault and defineSecret

`defineVault({ name })` declares a vault.
Its bound `Vault` has `reference` and `read(name, { version })`.
`defineSecret({ name, generated })` declares a secret, and `context.resources.get` answers its `Secret`.

| Option                | What it declares                                                                                |
| --------------------- | ----------------------------------------------------------------------------------------------- |
| `name`                | the name within the package, which a stack binds                                                |
| `generated.vault`     | the package's vault the space generates the secret in at install, instead of a stack binding it |
| `generated.algorithm` | `"ES256"`: an ECDSA P-256 private key, as a JSON Web Key                                        |

### Secret

A `Secret` reads the version its deployment captured.
`Secret.of(value)`, from `@destack/resource`, makes a one-version secret from a value.

| Verb                          | What it does                                                                                                              |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `read()`                      | reads the captured version as `{ version, value }`, its value `{ encoding: "text" \| "base64", value }` of at most 64 KiB |
| `versions()`                  | reads the captured version and the earlier ones still retiring, newest first                                              |
| `SecretReading.text(reading)` | reads a text value, refusing a base64 one                                                                                 |

## Examples

### Store a credential

An installation stores a credential with `vault.put`.

```ts
import type { MethodContext } from "@destack/object";
import type { Identifier } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { vault } from "@destack/vault/object";

/** Store a GitHub token in the package's vault, as the installation the method runs for. */
export async function storeToken(
    call: MethodContext,
    keys: ObjectReference,
    secret: Identifier<"secret">,
    token: string,
) {
    // store the token as the only version
    const target = vault.target({ scope: keys.scope, id: vault.identifier(keys.id) });
    const value = { encoding: "text", value: token } as const;
    await call.mutate(call.caller, [
        vault.call("put", { ...target, secret, name: "github", value }),
    ]);
}
```

### Rotate a secret

`rotate` makes a new version current, and the old one stays readable for `expiresIn`.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type vaultPackage from "@destack/vault/package";

/** Rotate a webhook secret to a new value, the old one readable for a week. */
export async function rotateWebhookSecret(
    client: ClientOf<typeof vaultPackage>,
    scope: string,
    id: Identifier<"secret">,
    value: string,
    requestId: string,
) {
    await client.secret.rotate({
        scope,
        id,
        requestId,
        value: { encoding: "text", value },
        expiresIn: { days: 7 },
    });
}
```

### Verify with every readable version

`versions` reads every readable version, so older signatures still verify.

```ts
import { type Secret, SecretReading } from "@destack/resource";

/** Report whether a webhook's token matches any readable version of its secret. */
export async function isKnownToken(secret: Secret, token: string): Promise<boolean> {
    const readings = await secret.versions();

    return readings.some((reading) => SecretReading.text(reading) === token);
}
```

### Bind secrets in a stack

A stack declares vaults and secrets, and binds them to installations.

```ts
import { defineStack } from "@destack/stack";
import { defineVault } from "@destack/vault";
import vaultPackage from "@destack/vault/package";

// declare a vault kept 30 days after deletion, and a GitHub secret
const pushVault = defineVault({ name: "push" });
export const personal = defineStack({
    types: [vaultPackage],
    vaults: { push: pushVault.provision({ retention: { within: { days: 30 } } }) },
    secrets: { github: { parent: "push", name: "github" } },
});
```

## Comparison

### GCP Secret Manager

Secrets, versions, their states and the current version basically follow GCP Secret Manager.
Deployments read the version they captured.

| Secret Manager                      | `@destack/vault`                         | Difference                                                               |
| ----------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------ |
| `Secret`                            | `secret`                                 | Lives in a `vault` of a space, an account or an organization.            |
| `SecretVersion`                     | `version`                                | The same, its value bound to its secret and version.                     |
| `ENABLED`, `DISABLED`, `DESTROYED`  | `"enabled"`, `"disabled"`, `"destroyed"` | The same.                                                                |
| the `latest` alias, version aliases | `currentVersion`                         | One alias, which reads take by default.                                  |
| `AddSecretVersion`                  | `vault.put`, `secret.rotate`             | `put` destroys the earlier versions, `rotate` expires the old one later. |
| `expire_time` of a secret           | `expiresAt` of a version                 | Each version expires on its own.                                         |
| `DeleteSecret`                      | `delete`, then `purge`                   | A deleted secret stays restorable for 30 days.                           |
| `AccessSecretVersion`               | `Secret.read`                            | Reads the version the deployment captured.                               |
