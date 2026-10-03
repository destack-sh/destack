# @destack/relay

Relay HTTP requests for Destack names to the hosts and regions serving them.

## Names

The relay forwards a request for a name to the cell serving it, through the host's tunnel or to the region.

```text
notes.personal.flotothemoon.destack.space             the cell serving the space personal of flotothemoon
notes.feature-x--personal.flotothemoon.destack.space  the same cell, for the branch feature-x
macbook.flotothemoon.destack.computer                 the host macbook of flotothemoon
```

## Relays

`RelayServer.start` serves the relay's names and its hosts' tunnels on one listener.

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

`TunnelClient.open` opens a host's tunnel to the relay and dials again with backoff whenever the tunnel ends.

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

A tunnel caps each frame payload, the requests open per host and the wait for a response head, and closes when the relay refuses a token renewal.

```ts
export const MAX_PAYLOAD_BYTES = 64 * 1024;
export const MAX_STREAMS = 100;
const ANSWER_TIMEOUT_MILLISECONDS = 100_000;
```

## Protocol

A tunnel sends one yamux frame per binary WebSocket message, and the relay opens the even streams.

```text
byte 0      type: data, window, ping, go-away
byte 1      flags: SYN, ACK, FIN, RST
bytes 2-5   stream
bytes 6-9   window delta, ping value or go-away code
bytes 10-   payload: body bytes, or the request or response head of a SYN or ACK
```

## Handshake

A host sends its token in base64url as a second WebSocket subprotocol after `destack.tunnel`, and the relay verifies the token and answers with `destack.tunnel` alone.

```http
GET /tunnel HTTP/1.1
Upgrade: websocket
Sec-WebSocket-Protocol: destack.tunnel, destack.bearer.<token>

HTTP/1.1 101 Switching Protocols
Sec-WebSocket-Protocol: destack.tunnel
```
