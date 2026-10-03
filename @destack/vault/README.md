# @destack/vault

Declare, store and read versioned secrets.

## Declarations

`defineVault` declares a vault, `defineSecret` declares a secret the package reads, and `read` returns the secret's value and version.

```ts
import { defineSecret, defineVault } from "@destack/vault";

export const credentials = defineVault({ name: "credentials", spec: {} });
export const githubToken = defineSecret({ name: "github-token" });

const { value, version } = await githubToken.get(context).read();
```

## Bindings

`secrets` names a stack's secrets by vault and name, and an installation binds each secret declaration to one of them, optionally at a fixed `version`.

```ts
export const personal = defineSpace({
    resources: {
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    secrets: { github: { vault: "credentials", name: "github" } },
    installations: { notes: install(notes, { "github-token": { secret: "github", version: 2 } }) },
});
```

## Captured versions

A deployment captures the version a secret binding names, or the current version when the binding names none.

```ts
install(notes, { "github-token": { secret: "github", version: 2 } }); // version 2
install(notes, { "github-token": { secret: "github" } }); // the current version at deployment
```

## Runtime

`Secret` reads a captured version through a `SecretClient` authenticated as the installation, and `context.bind` gives it to the secret declaration.

```ts
import { spaceService } from "@destack/space/service";
import { Secret } from "@destack/vault";
import { SecretClient } from "@destack/vault/object";

const client = new SecretClient(spaceService, { url: vaultUrl, headers: installationHeaders });
context.bind(githubToken, new Secret(client, capture));
```

## Objects

`secret.create` creates a secret in a vault, and `version.create` writes a new version of it.

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
```

## Account connections

`RemoteVault` stores the OAuth credentials of account connections as secrets in space vaults.

```ts
import { RemoteVault } from "@destack/vault/account";

const connections = new Connections({
    providers,
    vault: new RemoteVault(
        async (spaceId, subject) =>
            new SecretClient(spaceService, {
                url: vaultUrl(spaceId),
                headers: await vaultHeaders(spaceId, subject),
            }),
    ),
});
```

## Hosting

`LocalKeyring.open` reads the host's root keys from its keychain, and `servedObjects` and `vaultProvider` take the same keyring.

```ts
import { LocalKeyring } from "@destack/host/keychain";
import { vaultProvider } from "@destack/vault/provider";
import { servedObjects } from "@destack/vault/server";

const keyring = await LocalKeyring.open(keychain, hostId);
const objects = servedObjects(keyring, location, { days: 30 });
const provider = vaultProvider(database, keyring, location);
```

## Root key rotation

On a stopped host, `LocalKeyring.rotate` adds a new active root key, `VaultKey.rewrapAll` wraps the old key's vault keys under it, and `VaultKey.retire` removes the old key.

```ts
const rotated = await LocalKeyring.rotate(keychain, hostId);
await VaultKey.rewrapAll(database, rotated, location, previousKeyId);
await VaultKey.retire(database, keychain, hostId, previousKeyId);
```
