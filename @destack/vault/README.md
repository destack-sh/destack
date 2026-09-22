Declare, store and access versioned secrets.

## Usage

```ts
import { defineSecret, defineVault } from "@destack/vault/declare";

export const credentials = defineVault({ name: "credentials", spec: {} });
export const githubToken = defineSecret({ name: "github-token" });

const { value, version } = await githubToken.get(context).read();
```

```ts
import { connect, BoundSecret } from "@destack/vault/client";

const client = connect({ url: vaultUrl, headers: authenticatedHeaders });
const token = new BoundSecret(client, { space: spaceId, secret: secretId });
const { value, version } = await token.read();
```

## Administration

```ts
import { createRequestId } from "@destack/service/request";

const secret = await client.secret.create({
    spaceId,
    vaultId,
    name: "github",
    requestId: createRequestId(),
});
const written = await client.version.write({
    spaceId,
    secretId: secret.id,
    revision: secret.revision,
    requestId: createRequestId(),
    value: { encoding: "text", value: credential },
    promote: true,
});
```

## Hosting

```ts
import { LocalKeyring, EnvelopeEncryption } from "@destack/vault/encryption";
import { implementService } from "@destack/vault/server";
import { Vault } from "@destack/vault/vault";
import { Server } from "@destack/service/server";
import { Health } from "@destack/service/health";
import { defineService } from "@destack/service";
import { vaultService } from "@destack/vault/service";

const keys = await LocalKeyring.import("2026-09", rootKeys);
const vault = new Vault(database, new EnvelopeEncryption(keys), "eu");
const server = await Server.start({
    ...implementService(vault),
    audience: receivingPackageId,
    spaceId: regionalSystemSpaceId,
    resources,
    health: new Health("vault"),
    authenticate, // verify the credential and return Caller
    authorizeHost: authorizeInstallation,
    drainTimeout: 10000,
    dispose: () => database.close(),
});

export const service = defineService(
    { name: "vault", version: 1, protocol: "http", handler: "fetch" },
    vaultService,
);

export function fetch(request: Request): Promise<Response> {
    return server.fetch(request);
}

// invoke with a separately authorized maintenance client
await client.secret.purge({ spaceId, limit: 100 });
await client.request.rewrap({ spaceId, keyId: "2026-08", limit: 100 });
```
