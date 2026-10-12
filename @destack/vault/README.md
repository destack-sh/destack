# @destack/vault

`@destack/vault` keeps secrets as HashiCorp Vault's KV v2 engine does, encrypted under versioned vault keys as its Transit engine keeps them.

```ts
export const githubToken = defineSecret({ name: "github-token" }); // a KV v2 path the package reads
await Secret.text(githubToken.get(context.resources)); // vault kv get
await credentials.get(context.resources).read("github", { version: 2 }); // vault kv get -version=2
await client.version.create({ scope, parentId: secret.id, value, requestId }); // vault kv put
await client.version.create({ scope, parentId: secret.id, value, stage: "pending", requestId }); // AWSPENDING
await client.vaultKey.rotate({ scope, parentId: vault.id, requestId }); // vault write -f transit/keys/x/rotate
```

## Keys

A vault starts with its first key version, each rotation adds the next, and the key material never leaves the vault service.

```ts
const keys = await client.vaultKey.list({ scope, where: { parentId: vault.id } }); // numbers, never material
```

## Declarations

`defineVault` declares a vault and `defineSecret` a secret the package reads, and a secret declaring `generated` gets its first version generated in the package's vault.

```ts
import { defineSecret, defineVault } from "@destack/vault/declare";

export const pushVault = defineVault({ name: "push", spec: {} });
export const pushKey = defineSecret({
    name: "push-key",
    generated: { vault: pushVault.name, algorithm: "ES256" },
});
```

## Bindings

A stack lists its secrets by vault and name, and an installation binds a declaration to one at a fixed version or the current one.

```ts
export const personal = defineStack({
    resources: {
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    secrets: { github: { vault: "credentials", name: "github" } },
    installations: { site: install(site, { "github-token": { secret: "github", version: 2 } }) },
});
```

## Service

`implementVault` serves a space's vaults, secrets and versions, and `VaultKeystore.reencrypt` moves the vault keys under the keyring's active root key.

```ts
import { implementVault } from "@destack/vault/server";

const vault = implementVault({
    database,
    keyring: await LocalKeyring.read(rootKey),
    machine: machineId,
});
await new VaultKeystore(database, rotated, machineId).reencrypt(previous.active);
```

## Consumers

A machine service puts its own credentials into a space's vault with `vault.put`, one live version each, bound to it as their consumer, and reads them back as itself with `secret.read`.

```ts
await store.mutate(caller, {
    id,
    calls: [
        vault.call("put", {
            ...vault.target(reference),
            secret: secretId,
            value,
            consumer: machine,
        }),
    ],
});
await store.query(
    Caller.of(machine, now),
    secret.call("read", { ...secret.target(reference), version }),
);
await store.mutate(caller, {
    id,
    calls: [
        vault.call("remove", { ...vault.target(reference), secret: secretId, consumer: machine }),
    ],
});
```

## Operators

`Secret.of` reads a value an operator sets, such as an environment variable, as a secret with one version.

```ts
const stripe = new StripeClient(Secret.of(stripeKey), fetch);
```

## Tables

`vaultTables` lists the tables a space database includes.

```ts
import { vaultTables } from "@destack/vault/stack";

const space = defineDatabase({ name: "space", tables: [...vaultTables, journal] });
```
