# @destack/connection

`@destack/connection` keeps the credentials of external accounts in vaults and applies them to outbound requests, as Nango's integrations, connections and proxy do.

```ts
// a Nango integration
await client.integration.create({
    scope,
    requestId,
    issuer: { kind: "server", url: "https://auth.example.com" },
    endpoints,
    clientId,
    authentication: "client_secret_basic",
    redirectUris,
    vault,
    secret,
    visibility: "private",
});
// a Nango connection
await client.connection.create({
    scope,
    requestId,
    protocol: { kind: "authorization-code" },
    resource,
    vault,
});
// Nango's auth flow
const { url } = await client.connection.authorize({ scope, id, requestId, redirectUri });
const lent = await ConnectionRequest.borrow(home, reference, request, Date.now()); // Nango's proxy
await client.connection.revoke({ scope, id, requestId }); // RFC 7009 revocation
```

## Connections

A `connection` keeps the credential of one external account in a vault of its scope, authenticating by its `protocol`'s kind.

```ts
const { id } = await client.connection.create({
    scope: spaceId,
    requestId: RequestId.create(),
    // or signature, sigv4, jwt, authorization-code, client-credentials, device-code,
    // token-exchange, mtls
    protocol: { kind: "key", header: "x-api-key" },
    material: { kind: "key", key },
    vault: vault.reference(spaceId, vaultId),
});
```

## Dynamic registration

A connection to a `resource` without an integration registers a client at the resource's authorization server, as MCP clients do (RFC 9728, RFC 8414, RFC 7591).

```ts
await client.connection.create({
    scope,
    requestId,
    protocol: { kind: "authorization-code" },
    resource: "https://mcp.example.com/mcp",
    vault,
});
```

## Server

`@destack/connection/server` serves connections and integrations as the connection installation, whose controller obtains and refreshes every token before it expires.

```ts
import server from "@destack/connection/server";

const extension = server(context); // tokens through the host's network, the home through its egress
```

## Borrowing

`ConnectionRequest.borrow` lends a connection to a bound installation's outbound request, applying its current credential.

```ts
const { request, certificate } = await ConnectionRequest.borrow(home, reference, outbound, Date.now());
```

## Installations

An app installed on external accounts gets its tokens from an installation of the package keeping the app's key, which declares the `Issuer` template.

```ts
import { Issuer } from "@destack/connection/object";

// the issuing package, such as the forge with its GitHub App
export const issuer = defineObject(Issuer.definition()).methods(Issuer.methods);

// the integration records the installation and its issuer, whose consumer the connection installation is
await client.integration.create({
    scope,
    requestId,
    issuer: { kind: "installation", package: Package.of(forge), name: "forge", issuer: reference },
    endpoints: {
        installation: "https://github.com/apps/destack/installations/new",
        isIssuerReturned: false,
    },
    clientId,
    authentication: "none",
    redirectUris,
});
```

## Tables

`connectionTables` lists the tables of connections and their pending authorizations, and `integrationTables` lists the integrations.

```ts
import { connectionTables, integrationTables } from "@destack/connection/stack";

const space = defineDatabase({
    name: "main",
    tables: [...connectionTables, ...integrationTables],
});
```
