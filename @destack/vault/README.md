Declare, store and read versioned secrets.

## Usage

A package declares a vault and the secrets it reads.

```ts
export const credentials = defineVault({ name: "credentials", spec: {} });
export const githubToken = defineSecret({ name: "github-token" });

const { value, version } = await githubToken.get(context).read();
```

## Bindings

A stack binds each secret declaration to one of its secrets, optionally pinned to a version.

```ts
export const personal = defineSpace({
    resources: { credentials: { declaration: credentials, retention: "retain", tags: {} } },
    secrets: { github: { vault: "credentials", name: "github" } },
    installations: { notes: install(notes, { "github-token": { secret: "github", version: 2 } }) },
});
```

An installation reads the version each live deployment captured.

| Binding | Captured version |
|---|---|
| pinned | that version |
| unpinned | the current version when the deployment was created |

## Runtime

The runtime binds a captured secret through a vault client authenticated as the installation.

```ts
context.bind(
    githubToken,
    new Secret(connect({ url: vaultUrl, headers: installationHeaders }), capture),
);
```

## Objects

| Object | Methods |
|---|---|
| `vault` | `get`, `list` |
| `secret` | `get`, `list`, `create`, `update`, `disable`, `enable`, `promote`, `read`, `delete`, `restore`, `purge` |
| `version` | `get`, `list`, `create`, `disable`, `enable`, `destroy` |

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
const connections = new Connections({
    providers,
    vault: new RemoteVault(async (spaceId, subject) =>
        connect({ url: vaultUrl(spaceId), headers: await vaultHeaders(spaceId, subject) }),
    ),
});
```

## Hosting

A host keeps its root keys in a keychain and shares one `ValueStore` between the service and the provider.

```ts
const values = new ValueStore(await LocalKeyring.open(keychain, hostId), hostId);
const service = implementService({ database, values, recovery: { days: 30 } });
const provider = vaultProvider(database, values);
```

A stopped host rotates its root key: it adds an active key, rewraps the vault keys under the old one, then retires it.

```ts
const rotated = new ValueStore(await LocalKeyring.rotate(keychain, hostId), hostId);
await rotated.rewrapUnder(database, previousKeyId);
await ValueStore.retire(database, keychain, hostId, previousKeyId);
```
