Relay HTTP requests for Destack names to the cells serving them.

## Names

Each name goes to a cell.

| Name | Cell |
|---|---|
| `notes.personal.flotothemoon.destack.space` | the host or region serving the space `personal` of `flotothemoon` |
| `notes.feature-x--personal.flotothemoon.destack.space` | the same cell, for the branch `feature-x` |
| `macbook.flotothemoon.destack.computer` | the host `macbook` of `flotothemoon` |

## Relay

`RelayServer` runs a relay in one process.

```ts
const relay = RelayServer.start({ origin: "https://relay.destack.space", listener: { hostname: "0.0.0.0", port: 443, tls }, database, resolver: new ResolverCache(database), tokens: universe(RELAY_PACKAGE.id), certificate: { cert, key }, report });
```

## Tunnels

A host keeps its tunnel open with `TunnelClient`.

```ts
const tunnel = TunnelClient.open({ url: "https://relay.destack.space/tunnel", token: async () => (await identity.token(RELAY_PACKAGE.id, hosts, fetch)).accessToken, fetch: (request) => views.fetch(request, "relay"), report });
```

## Limits

Each tunnel has these limits.

| Limit | Value | Answer |
|---|---|---|
| frame payload | 64 KiB | the connection closes |
| open requests per host | 100 | `503` |
| wait for a response head | 100 s | `504` |
| token renewal | every 15 s | a refused renewal closes the connection |

## Protocol

A connection is a WebSocket with one yamux-shaped frame per binary message.

| Bytes | Field |
|---|---|
| 0 | type: data, window, ping, go-away |
| 1 | flags: SYN, ACK, FIN, RST |
| 2–5 | stream; the relay opens even streams |
| 6–9 | window delta, ping value or go-away code |
| 10– | payload: body bytes, or the head a SYN or ACK carries |
