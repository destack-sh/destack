# @destack/connection

`@destack/connection` is Nango: an `integration` is a Nango integration, the OAuth client registered at a provider, a `connection` is a Nango connection holding one external account's credential in a vault, and `Lender.lend` is Nango's proxy, applying the credential to an outbound request and refreshing it before it expires.

```ts
// a Nango integration
await client.integration.create({
    scope,
    requestId,
    issuer,
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
const loan = await lender.lend(reference, request, Date.now()); // Nango's proxy
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

## Service

`implementConnections` serves a space's connections, and its integrations where the space is their home, with the `lender` the machine's egress uses.

```ts
import { implementConnections } from "@destack/connection/server";

const connections = implementConnections({
    space: spaceId,
    consumer: machine,
    fetch,
    issuers: [],
    home,
});
const lender = connections.lender(server);
```

## Installations

`GitHubIssuer` issues the installation tokens of a GitHub App, signing its JWT with `Assertion.sign`.

```ts
import { GitHubIssuer } from "@destack/connection/github";

const github = new GitHubIssuer({ fetch });
const token = await github.token(appId, "4242", privateKey, { permissions: { metadata: "read" } });
await github.request("GET", "/repos/acme/site", token.accessToken);
```

## Tables

`connectionTables` lists the tables of connections and their pending authorizations, and `integrationTables` lists the integrations.

```ts
import { connectionTables, integrationTables } from "@destack/connection/stack";

const space = defineDatabase({
    name: "space",
    tables: [...connectionTables, ...integrationTables],
});
```
