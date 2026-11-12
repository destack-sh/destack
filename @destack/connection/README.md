# @destack/connection

Connect external accounts once, and call their APIs with credentials kept in a vault.

## Overview

A package declares the external API it calls, and the connection installation keeps the account's credential and applies it to each outbound call.

- **Connection**: the credential of one external account, authenticating by its `protocol`; `authorize`, `complete`, `rotate` and `disconnect`; its controller refreshes every token before it expires.
- **Integration**: an external app's OAuth client, its endpoints and secret; `register` creates one by dynamic client registration.
- **Issuer**: a package keeping an app's key, such as a GitHub App, which issues tokens for the accounts the app is installed on.

```ts
import type connectionPackage from "@destack/connection/client";
import { defineConnection } from "@destack/connection/declare";
import type { ClientOf } from "@destack/object/client";
import type { ExternalClient } from "@destack/resource";
import type { Identifier } from "@destack/schema";

export const github = defineConnection({
    name: "github",
    spec: { resource: "https://api.github.com", scopes: ["repo"] },
});

async (
    client: ClientOf<typeof connectionPackage>,
    scope: string,
    id: Identifier<"connection">,
    api: ExternalClient,
    requestId: string,
) => {
    const { url } = await client.connection.authorize({ scope, id, requestId });
    const parameters = { state: "…", code: "…" };
    await client.connection.complete({ scope, id, parameters, requestId });
    const response = await api.fetch("/user/repos");
};
```

## Examples

### Call an API from a server

A server calls a bound connection's API below its address, and the host's egress applies the credential.

```ts
import type { ExternalClient } from "@destack/resource";

async (stripe: ExternalClient) => {
    const response = await stripe.fetch("/v1/customers", { method: "GET" });
    const customers = await response.json();
};
```

### Supply an API key

A connection with the `key` protocol sends the key its owner supplies in a header.

```ts
import type connectionPackage from "@destack/connection/client";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

async (
    client: ClientOf<typeof connectionPackage>,
    scope: string,
    id: Identifier<"connection">,
    requestId: string,
) => {
    const material = { kind: "key", key: "sk_live_…" } as const;
    await client.connection.rotate({ scope, id, material, requestId });
};
```

### Sign HTTP messages

`MessageSignature.sign` signs a request's method, target and content digest after RFC 9421.

```ts
import { MessageSignature } from "@destack/connection/signature";
import { BunEnvironment } from "@destack/environment/bun";

const { crypto } = BunEnvironment.system;
const request = new Request("https://api.example.com/v1/orders", { method: "POST", body: "{}" });
const signer = { algorithm: "hmac-sha256", keyId: "key-1", key: "c2VjcmV0" } as const;
const signed = await MessageSignature.sign(request, signer, Date.now(), crypto);
```

## Comparison

### Nango

Integrations, connections and the proxy that applies credentials follow Nango; Destack's differ in three ways.

- A package declares the API it calls with `defineConnection`, and the connection installation binds it, so the package never reads the credential.
- Credentials and client secrets live in the vault the stack binds as `credentials`, sealed at rest.
- A connection to an API without an integration registers a client at its authorization server, as MCP clients do after RFC 9728, RFC 8414 and RFC 7591.
