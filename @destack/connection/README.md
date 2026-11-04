# @destack/connection

`@destack/connection` keeps the credentials of external accounts in its bound vault and applies them to outbound requests, as Nango's integrations, connections and proxy do.

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
    secret,
    isPublic: false,
});
// a Nango connection
await client.connection.create({
    scope,
    requestId,
    protocol: { kind: "authorization-code" },
    resource,
});
// Nango's auth flow
const { url } = await client.connection.authorize({ scope, id, requestId, redirectUri });
const leased = await ConnectionRequest.lease(primary, reference, request, clock.now(), crypto); // Nango's proxy
await client.connection.revoke({ scope, id, requestId }); // RFC 7009 revocation
```

## Connections

A `connection` keeps the credential of one external account, authenticating by its `protocol`'s kind.

```ts
const { id } = await client.connection.create({
    scope: spaceId,
    requestId: RequestId.create(environment),
    // or signature, sigv4, jwt, authorization-code, client-credentials, device-code,
    // token-exchange, mtls
    protocol: { kind: "key", header: "x-api-key" },
    material: { kind: "key", key },
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
});
```

## Server

`@destack/connection/server` serves connections and integrations as the connection installation, whose controller obtains and refreshes every token before it expires.
The installation keeps every credential and client secret in the vault its stack binds as `credentials`.

```ts
import server from "@destack/connection/server";
import { connectionVault } from "@destack/connection/stack";

const extension = server(context); // tokens through the host's network, the primary through its egress

// an account home's stack: the vault, bound to the connections
const home = {
    vaults: { credentials: connectionVault.declare() },
    installations: {
        connection: install(connectionPackage, { credentials: "credentials" }),
    },
};
```

## Leasing

`ConnectionRequest.lease` applies a connection's current credential, leased at its primary, to a bound installation's outbound request.

```ts
const { request, certificate } = await ConnectionRequest.lease(
    primary,
    reference,
    outbound,
    clock.now(),
    crypto,
);
```

## Installations

An app installed on external accounts gets its tokens from an installation of the package keeping the app's key, which declares the `Issuer` template.

```ts
import { Issuer } from "@destack/connection/object";

// the issuing package, such as the forge with its GitHub App
export const issuer = defineObject(Issuer.definition(access), Issuer.methods); // access: who reads, manages and calls it

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

`connectionTables` lists the tables of connections and their pending authorizations, `integrationTables` lists the integrations, and `connectionVault` declares the vault of their credentials.

```ts
import { connectionTables, integrationTables } from "@destack/connection/stack";

const space = defineDatabase({
    name: "main",
    tables: [...connectionTables, ...integrationTables],
});
```
