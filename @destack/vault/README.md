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

### Bindings

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

### Generated secrets

`generated` names one of the package's vaults and an algorithm, and a space installing the package creates the secret there and writes its first version once the vault's key exists: `ES256` writes an ECDSA P-256 private key as a JWK.

```ts
export const pushVault = defineVault({ name: "push", spec: {} });
export const pushKey = defineSecret({
    name: "push-key",
    generated: { vault: pushVault.name, algorithm: "ES256" },
});
```

### Captured versions

A deployment captures the version a secret binding names, or the current version when the binding names none.

```ts
install(notes, { "github-token": { secret: "github", version: 2 } }); // version 2
install(notes, { "github-token": { secret: "github" } }); // the current version at deployment
```

### Runtime

A workload's secret declarations and vaults connect through its space's service at the host's egress as the installation: a secret to the version its deployment captured, a vault to a `SecretClient`.

```ts
const { value, version } = await githubToken.get(context.resources).read();
const secrets = vault.get(context.resources); // a SecretClient of the space's service
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

## Hosting

`LocalKeyring.open` reads the host's root keys from its keychain, and `serveSecrets` and `KeyringVaultHost` take the same keyring.

```ts
import { LocalKeyring } from "@destack/host/keychain";
import { KeyringVaultHost, vaultProvider } from "@destack/vault/provider";
import { serveSecrets } from "@destack/vault/server";

const keyring = await LocalKeyring.open(keychain, hostId);
const objects = serveSecrets(keyring, location, { days: 30 });
// keep the vaults' keys in the cell's database
const provider = vaultProvider(new KeyringVaultHost(database, keyring, location));
```

### Root key rotation

On a stopped host, `LocalKeyring.rotate` adds a new active root key, `VaultKey.rewrapAll` wraps the old key's vault keys under it, and `VaultKey.retire` removes the old key.

```ts
const rotated = await LocalKeyring.rotate(keychain, hostId);
await VaultKey.rewrapAll(database, rotated, location, previousKeyId);
await VaultKey.retire(database, keychain, hostId, previousKeyId);
```

## Account connections

`RemoteVault` stores the OAuth credentials of account connections as secrets in space vaults.

```ts
import { RemoteVault } from "@destack/vault/account";

const connections = {
    providers,
    vault: new RemoteVault(
        async (spaceId, subject) =>
            new SecretClient(spaceService, {
                url: vaultUrl(spaceId),
                headers: await vaultHeaders(spaceId, subject),
            }),
    ),
};
```

## Tables

`vaultKey` keeps each vault's key wrapped under the holding host's root key, beside the `vault`, `secret` and `secretVersion` object tables a space's database includes.

```ts
import { vaultKey } from "@destack/vault/stack";

const [kept] = await database.select().from(vaultKey).where(eq(vaultKey.vaultId, vaultId));
```

## Errors

A failed unwrap or decryption throws a `VaultError` with a stable code and without the secret's contents, which callers see as an internal server error.

```ts
import { VaultError } from "@destack/vault/error";

if (error instanceof VaultError && error.code === "KEY_UNAVAILABLE") {
    report(error.message); // the value is wrapped under another vault key
}
```

## Tests

`VaultFixture` provisions a space with a vault, its key and a member role, and serves the vault's objects.

```ts
import { VaultFixture } from "@destack/vault/test";

await using fixture = await VaultFixture.open("sqlite");
const { first } = await fixture.createSecret();
```
