Declare, store and read versioned secrets.

## Declarations

A package declares a vault and the secrets it reads.

```ts
import { defineSecret, defineVault } from "@destack/vault";

export const credentials = defineVault({ name: "credentials", spec: {} });
export const githubToken = defineSecret({ name: "github-token" });

const { value, version } = await githubToken.get(context).read();
```

## Bindings

A stack binds each secret declaration to one of its secrets, optionally pinned to a version.

```ts
export const personal = defineSpace({
    resources: {
        credentials: { declaration: credentials, retention: { within: { days: 30 } }, tags: {} },
    },
    secrets: { github: { vault: "credentials", name: "github" } },
    installations: { notes: install(notes, { "github-token": { secret: "github", version: 2 } }) },
});
```

A live deployment captures the version each of its secrets reads.

| Binding  | Captured version                                    |
| -------- | --------------------------------------------------- |
| pinned   | that version                                        |
| unpinned | the current version when the deployment was created |

## Runtime

The runtime binds a captured secret through a vault client authenticated as the installation.

```ts
import { spaceService } from "@destack/space/service";
import { Secret } from "@destack/vault";
import { SecretClient } from "@destack/vault/object";

const client = new SecretClient(spaceService, { url: vaultUrl, headers: installationHeaders });
context.bind(githubToken, new Secret(client, capture));
```

## Objects

A space's vault service serves vaults, their secrets and each secret's versions.

An administrator creates a secret, then writes its first version.

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

`RemoteVault` keeps connections' OAuth credentials as secrets in space vaults.

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

A host keeps its root keys in a keychain and shares one keyring between the served objects and the provider.

```ts
import { LocalKeyring } from "@destack/vault/encryption";
import { vaultProvider } from "@destack/vault/provider";
import { servedObjects } from "@destack/vault/server";

const keyring = await LocalKeyring.open(keychain, hostId);
const objects = servedObjects(keyring, location, { days: 30 });
const provider = vaultProvider(database, keyring, location);
```

A stopped host rotates its root key by adding an active key, rewrapping the old key's vault keys under it, then retiring the old key.

```ts
const rotated = await LocalKeyring.rotate(keychain, hostId);
await VaultKey.rewrapAll(database, rotated, location, previousKeyId);
await VaultKey.retire(database, keychain, hostId, previousKeyId);
```
