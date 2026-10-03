Relay HTTP requests for Destack names to the hosts and regions serving them.

## Names

The relay sends each name to the cell serving it, through the host's tunnel or to the region.

| Name                                                   | Cell                                                              |
| ------------------------------------------------------ | ----------------------------------------------------------------- |
| `notes.personal.flotothemoon.destack.space`            | the host or region serving the space `personal` of `flotothemoon` |
| `notes.feature-x--personal.flotothemoon.destack.space` | the same cell, for the branch `feature-x`                         |
| `macbook.flotothemoon.destack.computer`                | the host `macbook` of `flotothemoon`                              |

## Relays

`RelayServer` serves a relay's names and its hosts' tunnels on one listener in one process.

```ts
import { Resolver } from "@destack/account/directory";
import { RelayServer } from "@destack/relay/bun";
import { RELAY_PACKAGE } from "@destack/relay/server";
import { TokenVerifier } from "@destack/service/authentication";

const relay = RelayServer.start({
    origin: "https://relay.destack.space",
    listener: { hostname: "0.0.0.0", port: 443, tls },
    database,
    resolver: Resolver.global(database),
    tokens: new TokenVerifier({
        authority: { kind: "universe" },
        issuer,
        audience: RELAY_PACKAGE.id,
        keys,
    }),
    certificate: { cert, key },
    report,
});
```

## Tunnels

A host keeps its tunnel open with `TunnelClient`, which dials again with backoff whenever the tunnel ends.

```ts
import { RELAY_PACKAGE } from "@destack/relay/server";
import { TunnelClient } from "@destack/relay/tunnel";

const tunnel = TunnelClient.open({
    url: "https://relay.destack.space/tunnel",
    token: async () => (await identity.token(RELAY_PACKAGE.id, accounts, fetch)).accessToken,
    fetch: (request) => views.fetch(request, "relay"),
    report,
});
```

## Limits

Each tunnel bounds its frame payload, its open requests per host and its wait for a response head, and closes when the relay refuses a token renewal.

## Protocol

A tunnel is a WebSocket carrying one yamux-shaped frame per binary message.

| Bytes | Field                                                 |
| ----- | ----------------------------------------------------- |
| 0     | type: data, window, ping, go-away                     |
| 1     | flags: SYN, ACK, FIN, RST                             |
| 2–5   | stream; the relay opens even streams                  |
| 6–9   | window delta, ping value or go-away code              |
| 10–   | payload: body bytes, or the head a SYN or ACK carries |

A host offers two subprotocols: `destack.tunnel`, and `destack.bearer.` followed by its token in base64url.
The relay verifies the token and answers with `destack.tunnel` alone.
