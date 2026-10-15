# @destack/vault

`@destack/vault` keeps a space's secrets in vaults as versions sealed at rest, staged and promoted as AWS Secrets Manager does.

```ts
await client.version.create({ scope, parentId: secret.id, value, requestId });
await client.version.create({ scope, parentId: secret.id, value, stage: "pending", requestId });
await client.secret.promote({ scope, id: secret.id, version: 2, requestId });
await client.secret.purge({ scope, id: secret.id, requestId }); // destroys every version's value
```

## Declarations

`defineVault` declares a vault and `defineSecret` a secret a package reads; a secret with `generated` gets its first version generated.

```ts
import { defineSecret, defineVault } from "@destack/vault/declare";

export const pushVault = defineVault({ name: "push", spec: {} });
export const pushKey = defineSecret({
    name: "push-key",
    generated: { vault: pushVault.name, algorithm: "ES256" },
});
await Secret.text(pushKey.get(context.resources));
```

## Bindings

A stack lists its secrets by vault and name, and an installation binds a declared secret to one at a fixed or the current version.

```ts
export const personal = defineStack({
    resources: {
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    secrets: { github: { vault: "credentials", name: "github" } },
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

A package puts its own credential into a vault with `vault.put`, reads it back with `secret.read`, and removes it with `vault.remove`.

```ts
await store.mutate(caller, {
    id,
    calls: [
        vault.call("put", { ...vault.target(reference), secret: secretId, name, value, consumer }),
    ],
});
await store.query(consumerCaller, secret.call("read", secret.target(reference)));
await store.mutate(caller, {
    id,
    calls: [vault.call("remove", { ...vault.target(reference), secret: secretId, consumer })],
});
```

## Operators

`Secret.of` turns a value an operator sets, such as an environment variable, into a secret with one version.

```ts
const stripe = new StripeClient(Secret.of(stripeKey), fetch);
```

## Tables

`vaultTables` lists the vault package's tables.

```ts
import { vaultTables } from "@destack/vault/stack";

const database = defineDatabase({ name: "main", tables: vaultTables });
```
