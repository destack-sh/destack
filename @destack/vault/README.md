# @destack/vault

`@destack/vault` keeps secrets of spaces, accounts and organizations in vaults as versions sealed at rest, after GCP Secret Manager: versions are added, enabled, disabled and destroyed, and a secret's `current` alias selects the version reads take by default.

```ts
const added = await client.version.create({ scope, parentId: secret.id, value, requestId });
await client.secret.update({ scope, id: secret.id, currentVersion: added.number, requestId });
await client.version.disable({ scope, id: added.id, requestId });
await client.version.destroy({ scope, id: added.id, requestId }); // erases its value
```

## Divergences

Three behaviors differ from GCP on purpose: a deleted secret stays restorable within a recovery window before its purge erases every value, `vault.put` destroys a consumer's earlier versions as it adds the new current one, and each version carries its own expiry, where GCP expires the whole secret.

```ts
await client.secret.delete({ scope, id, revision, requestId }); // restorable for 30 days
await client.secret.restore({ scope, id, requestId });
await client.version.create({ scope, parentId, value, expiresAt, requestId }); // unreadable after expiresAt
```

## Declarations

`defineVault` declares a vault and `defineSecret` a secret a package reads; a secret with `generated` gets its first version generated.

```ts
import { defineSecret, defineVault, Secret } from "@destack/vault";

export const pushVault = defineVault({ name: "push", spec: {} });
export const pushKey = defineSecret({
    name: "push-key",
    generated: { vault: pushVault.name, algorithm: "ES256" },
});
await Secret.text(pushKey.get(server.resources)); // in the package's server
```

## Bindings

A stack declares its vaults and its secrets by vault and name, and an installation binds a declared secret to one at a fixed or the current version.

```ts
export const personal = defineStack([vaultPackage], {
    vaults: { push: pushVault.declare({ retention: { within: { days: 30 } } }) },
    secrets: { github: { parent: "push", name: "github" } },
    installations: { site: install(site, { "github-token": { secret: "github", version: 2 } }) },
});
```

## Server

The package's server serves vaults, secrets and versions, and provides the space's vaults; `secretBindable` lets deployments capture secret versions.

```ts
import vaultServer, { secretBindable } from "@destack/vault/server";

const { store } = serveSpace({ servers: [vaultServer], bindables: [secretBindable], ...options });
```

## Consumers

Software bound to a vault reads and writes it: an installation puts its own credential into a vault with `vault.put`, becoming the secret's custodian and consumer, reads it back with `secret.read`, and removes it with `vault.remove`; software bound to one secret only reads that secret.

```ts
await store.mutate(caller, {
    id,
    calls: [vault.call("put", { ...vault.target(reference), secret: secretId, name, value })],
});
await store.query(consumerCaller, secret.call("read", secret.target(reference)));
await store.mutate(caller, {
    id,
    calls: [vault.call("remove", { ...vault.target(reference), secret: secretId })],
});
```

## Operators

`Secret.of` turns a value an operator sets, such as an environment variable, into a secret with one version.

```ts
const stripe = new StripeClient(Secret.of(stripeKey), fetch);
```

## Errors

`VaultError` is the package's error family, such as `VERSION_UNAVAILABLE` for a destroyed, disabled or expired version.

```ts
if (VaultError.is(error, "VERSION_UNAVAILABLE")) {
    /* read another version */
}
```

## Tables

`vaultTables` lists the vault package's tables.

```ts
import { vaultTables } from "@destack/vault/stack";

const database = defineDatabase({ name: "main", tables: vaultTables });
```
