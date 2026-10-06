# @destack/vault

Declare, store and read versioned secrets, each encrypted under its vault's key.

## Declarations

`defineVault` declares a vault, and `defineSecret` declares a secret the package reads.

```ts
import { defineSecret, defineVault } from "@destack/vault";

export const credentials = defineVault({ name: "credentials", spec: {} });
export const githubToken = defineSecret({ name: "github-token" });
```

## Bindings

A stack lists its secrets by vault and name, and an installation binds a declaration to one at a fixed version or the current one.

```ts
export const personal = defineSpace({
    resources: {
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    secrets: { github: { vault: "credentials", name: "github" } },
    installations: {
        notes: install(notes, { "github-token": { secret: "github", version: 2 } }), // { secret: "github" } for the current version
    },
});
```

## Generated secrets

A secret declaring `generated` is provisioned in the package's vault when its space installs the package, its first version generated there.

```ts
export const pushVault = defineVault({ name: "push", spec: {} });
export const pushKey = defineSecret({
    name: "push-key",
    generated: { vault: pushVault.name, algorithm: "ES256" }, // an ECDSA P-256 private key as a JWK
});
```

## Runtime

A workload reads the version its deployment captured through its space's vault service as its installation.

```ts
const { value, version } = await githubToken.get(context.resources).read();
const secrets = credentials.get(context.resources); // a SecretClient of the vault service
```

## Objects

`secret.create` creates a secret in a vault, and `version.create` writes a version that becomes current unless `promote` is false.

```ts
const secret = await client.secret.create({
    spaceId,
    parentId: vaultId,
    name: "github",
    requestId: RequestId.create(),
});
await client.version.create({
    spaceId,
    parentId: secret.id,
    value: { encoding: "text", value: credential },
    requestId: RequestId.create(),
});
const { value, version } = await client.secret.read({ spaceId, id: secret.id });
```

## Service

`implementVault` serves a cell's vaults, keeping each vault's key encrypted under the cell's keyring and each version's value encrypted under its vault's key.

```ts
import { implementVault } from "@destack/vault/server";

const vault = implementVault({
    database,
    callKey,
    directory,
    keyring: await LocalKeyring.read(rootKey),
    location: cellId, // which each vault key's encryption context names
    machine: null,
    cell: cellId,
    spaces, // the space service's uplink
});
```

## Moves

A vault's key moves with its space's rows, sealed to the target's recipient and encrypted under the target's keyring there.

```ts
const sealed = await source.rewrap.seal(row, recipient); // source and target are KeyringVaultHosts
const opened = await target.rewrap.open(sealed, recipient);
```

## Destruction

Destroying a vault deletes its key, which shreds every value it encrypted, and is refused while a secret still holds a value.

```ts
await host.destroy(record);
```

## Root key rotation

`KeyringVaultHost.reencrypt` moves the vault keys under the keyring's active root key, after which the earlier version can be retired.

```ts
const rotated = await LocalKeyring.rotate(keychain, machineId);
await new KeyringVaultHost(database, rotated, location).reencrypt(previous.active); // how many keys moved
await LocalKeyring.retire(keychain, machineId, previous.active);
```

## Account connections

`RemoteVault` keeps the account service's connection credentials as secrets in space vaults, named by their object references.

```ts
import { RemoteVault } from "@destack/vault/account";

const vault = new RemoteVault(async (spaceId, caller) => secretClient(spaceId, caller));
const reference = await vault.write({ vault: vaultReference, id, name: "github", value }, caller);
const credential = await vault.read(reference, caller);
```

## Tables

`vaultDatabase` keeps the vaults, their keys, secrets and versions, with copies of the spaces' rows their checks read.

```ts
import { vaultDatabase, vaultKey } from "@destack/vault/stack";

const [kept] = await database.select().from(vaultKey).where(eq(vaultKey.vaultId, vaultId)); // { id, ciphertext, … }
```

## Tests

`VaultFixture` provisions a space with a vault and serves its objects.

```ts
import { VaultFixture } from "@destack/vault/test";

await using fixture = await VaultFixture.open("sqlite");
const { first } = await fixture.createSecret();
```
