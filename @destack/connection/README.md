# @destack/connection

Connect external accounts once, and call their APIs with credentials kept in a vault.

## Overview

`@destack/connection` is a credential broker after Nango: declared APIs, connected accounts, and an egress that adds the credential to each call.

Your package declares and calls an API:

- `defineConnection`: declares an API and the scopes its calls need.
- `ExternalClient`: calls the API with `fetch`.

Connections hold the credentials:

- `connection`: one external account's credential.
- `ConnectionProtocol`: how a connection authenticates, such as an API key or an OAuth grant.
- `integration`: an external app's OAuth client.
- `issuer`: issues tokens for an app's installations, such as a GitHub App's.
- `MessageSignature`: signs HTTP messages, as in RFC 9421.
- `ConnectionError`: the reasons a connection fails, checked with `ConnectionError.is`.

Declare GitHub's API, then call it from your package's server:

```ts
import { defineConnection } from "@destack/connection/declare";
import { defineMount, definePackage } from "@destack/object";
import { implementPackage } from "@destack/object/server";

// declare the API and the scopes the calls need
const github = defineConnection({
    name: "github",
    spec: { resource: "https://api.github.com", scopes: ["repo"] },
});
const repositories = defineMount({ name: "repositories", path: "/repositories" });
const site = definePackage({ resources: [github], mounts: [repositories] });

export default implementPackage(site, (context) => {
    // read the bound connection
    const api = context.resources.get(github);

    return {
        mounts: {
            repositories: async () => await api.fetch("/user/repos"),
        },
    };
});
```

## Reference

### defineConnection

`defineConnection({ name, spec })` declares a connection, and a stack or a person binds it at install.
The bound `ExternalClient` has the API's `url` and `fetch(path, init)`.

| Spec field | What it is                                                        |
| ---------- | ----------------------------------------------------------------- |
| `resource` | the API's HTTPS base URL, the protected resource of RFC 8707      |
| `scopes`   | the scopes the calls need, as the authorization server names them |

### ConnectionProtocol

`protocol` sets how calls authenticate.
`rotate` takes the material of the supplied kinds.

| Protocol                                  | How calls authenticate                                    | Material                                         |
| ----------------------------------------- | --------------------------------------------------------- | ------------------------------------------------ |
| `{ kind: "key", header, scheme }`         | an API key in a header, such as `authorization: Bearer …` | `{ key }`                                        |
| `{ kind: "signature", algorithm, keyId }` | HTTP message signatures with `hmac-sha256` or `ed25519`   | `{ key }`                                        |
| `{ kind: "sigv4", region, service }`      | AWS Signature Version 4                                   | `{ accessKeyId, secretAccessKey, sessionToken }` |
| `{ kind: "jwt", issuer, subject }`        | a JWT assertion exchanged for a token (RFC 7523)          | `{ privateKey }`, a private JWK                  |
| `{ kind: "mtls" }`                        | a TLS client certificate (RFC 8705)                       | `{ certificate, privateKey }` in PEM             |
| `{ kind: "authorization-code" }`          | a token from the code grant with PKCE                     | none, from `authorize`                           |
| `{ kind: "client-credentials" }`          | a token from the client credentials grant                 | none                                             |
| `{ kind: "device-code" }`                 | a token from the device authorization grant (RFC 8628)    | none, from `authorize`                           |
| `{ kind: "installation" }`                | a token an issuer issues for an app's installation        | none, from `authorize`                           |
| `{ kind: "token-exchange", source }`      | a token exchanged for another connection's (RFC 8693)     | none                                             |

### MessageSignature

`MessageSignature.sign(request, signer, now, crypto)` signs a request's method, target URI and content digest (RFC 9421, RFC 9530).
A signer is `{ algorithm, keyId, key }`: an HMAC secret in base64url, or an Ed25519 private JWK.

## Examples

### Connect with OAuth

`authorize` answers the page a person approves on, and `complete` takes the redirect's query.

```ts
import type connectionPackage from "@destack/connection/client";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Start a connection's authorization, answering the page the person approves it on. */
export async function startConnecting(
    client: ClientOf<typeof connectionPackage>,
    scope: string,
    id: Identifier<"connection">,
    requestId: string,
) {
    const { url } = await client.connection.authorize({ scope, id, requestId });

    return url;
}

/** Complete a connection's authorization with the query of the redirect back. */
export async function finishConnecting(
    client: ClientOf<typeof connectionPackage>,
    scope: string,
    id: Identifier<"connection">,
    redirect: URL,
    requestId: string,
) {
    const parameters = Object.fromEntries(redirect.searchParams);
    await client.connection.complete({ scope, id, parameters, requestId });
}
```

### Supply an API key

`rotate` stores a new API key.

```ts
import type connectionPackage from "@destack/connection/client";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";

/** Rotate a Stripe connection to a new secret key. */
export async function rotateStripeKey(
    client: ClientOf<typeof connectionPackage>,
    scope: string,
    id: Identifier<"connection">,
    key: string,
    requestId: string,
) {
    await client.connection.rotate({ scope, id, material: { kind: "key", key }, requestId });
}
```

### Sign HTTP messages

`MessageSignature.sign` answers a signed copy of a request.

```ts
import { MessageSignature } from "@destack/connection/signature";
import { BunEnvironment } from "@destack/environment/bun";

// sign an order's post with an HMAC secret
const { crypto } = BunEnvironment.system;
const request = new Request("https://api.example.com/v1/orders", { method: "POST", body: "{}" });
const signer = { algorithm: "hmac-sha256", keyId: "key-1", key: "c2VjcmV0" } as const;
const signed = await MessageSignature.sign(request, signer, Date.now(), crypto);
```

## Comparison

### Nango

Integrations, connections and the credential proxy basically follow Nango.
Packages declare their APIs instead of naming connections, and never read a credential.

| Nango                                     | `@destack/connection`   | Difference                                                              |
| ----------------------------------------- | ----------------------- | ----------------------------------------------------------------------- |
| an integration                            | `integration`           | The same, or registered dynamically for one connection.                 |
| a connection                              | `connection`            | Kept in the stack's vault, refreshed by its controller.                 |
| `nango.proxy({ endpoint, connectionId })` | `api.fetch(path)`       | Bound by declaration, without naming the connection.                    |
| auth modes: OAuth, API key, basic, …      | `ConnectionProtocol`    | Adds AWS Signature Version 4, HTTP signatures, mTLS and token exchange. |
| the Connect UI                            | `authorize`, `complete` | The calling app shows the page.                                         |
