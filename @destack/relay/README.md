# @destack/relay

Relay HTTP requests for Destack names to the machines and regions serving them.

## Names

The relay forwards a request for a name to the cell serving it, through the machine's tunnel or to the region.

```text
notes.personal.flotothemoon.destack.space             the cell serving the space personal of flotothemoon
notes.feature-x--personal.flotothemoon.destack.space  the same cell, for the branch feature-x
macbook.flotothemoon.destack.computer                 the machine macbook of flotothemoon
```

## Service

`implementRelay` follows the account service's copies of accounts, machines, machine keys and zones, and its `Relay` routes names and admits tunnels over them into a `MemoryTunnelHost` or `DurableObjectTunnelHost`.

```ts
import { implementRelay, MemoryTunnelHost } from "@destack/relay/server";
import { BunRelay } from "@destack/relay/bun";

const tunnels = new MemoryTunnelHost();
const service = implementRelay({
    origin: "https://relay.destack.space",
    database: relayDatabase.get(context),
    identity, // the WorkloadIdentity following the account service with its workload token
    tokens, // the verifier of the universe's tokens for RELAY_PACKAGE
    tunnels,
    report,
});
const listener = BunRelay.listen(service.relay, tunnels, { hostname: "0.0.0.0", port: 443, tls });
```

## Cloudflare

A Worker keeps a `DestinationCache` in its isolate, admits tunnels with a `Relay` and hands each machine's WebSocket to its `DurableObjectTunnel` in the jurisdiction of its account's residency, which it calls over RPC and which answers liveness probes asleep and verifies renewals without a database.

```ts
import { DurableObjectTunnel, DurableObjectTunnelHost } from "@destack/relay/cloudflare";
import { DestinationCache, Relay } from "@destack/relay/server";

const destinations = new DestinationCache(); // per isolate: a kept name reads no database until its cell answers 421
const relay = new Relay({ ...options, database: () => connect(), destinations }); // connects only on a miss
const tunnels = new DurableObjectTunnelHost(environment.TUNNEL, { eu: "eu" }); // an EU account's machines' objects stay in the EU
const routed = await relay.route(request); // a name's response, or a machine's admission
return routed instanceof Response ? routed : tunnels.open(routed, request); // fetch with Destack-Admission

export class DurableObjectRelayTunnel extends DurableObjectTunnel {
    constructor(state: DurableObjectTunnelState, environment: Environment) {
        super(state, environment, { url, tokens, report }); // tokens: the universe's verifier, no database
    }
}
```

## Workload

`relayWorkload` runs one relay per universe over `relayDatabase` (its copies of the accounts, machines, machine keys and zones, and the journal of tunnels opening and closing), the `workloadIdentity` and `relayConfiguration`, whose `serve` hands the started relay to a runtime serving its WebSockets, absent where a Worker serves them.

```ts
const tunnels = new MemoryTunnelHost();
const resources = new ResourceContext()
    .bind(relayDatabase, database)
    .bind(workloadIdentity, identity)
    .bind(relayConfiguration, {
        origin,
        tokens,
        tunnels,
        serve: (service) => BunRelay.listen(service.relay, tunnels, listener),
    }); // operators bind the workload RELAY_ROLE, reading accounts, machines, machine keys and zones
```

## Tunnels

`TunnelClient.open` keeps a machine's tunnel open, probing its liveness each heartbeat, renewing its token at 80% of its lifetime and dialing again with backoff whenever it ends.

```ts
import { RELAY_PACKAGE } from "@destack/relay/server";
import { TunnelClient } from "@destack/relay/tunnel";

const tunnel = TunnelClient.open({
    url: "https://relay.destack.space/tunnel",
    token: async () => (await identity.token(RELAY_PACKAGE.id, accounts, fetch)).accessToken,
    fetch: (request) => gateway.relay(request), // the machine's space service, installations and views
    name: (name) => publish(`https://${name}`), // on every token renewal and after renames: laptop.florian.destack.computer
    heartbeat: 15_000, // a text ping the relay answers pong without waking
    report,
});
```

## Limits

A tunnel caps each frame payload, the streams open per machine, the bytes in flight per stream and per session (RFC 9113 6.9), and the wait for a response head; an edge keeps destinations for a minute.

```ts
export const MAX_PAYLOAD_BYTES = 64 * 1024;
export const MAX_STREAMS = 1000; // a live subscription for each tab of every visitor
export const WINDOW_BYTES = 256 * 1024; // per stream at first, growing to MAX_WINDOW_BYTES
export const SESSION_WINDOW_BYTES = 16 * 1024 * 1024; // across a session's streams, bounding what it buffers
const ANSWER_TIMEOUT_MILLISECONDS = 100_000;
const NAME_TTL_MILLISECONDS = 60_000;
const MAX_NAMES = 10_000;
```

## Audit

The relay records a machine opening its tunnel, as the machine, and closing it once the machine or its last standing key is revoked, as the relay, in the machine's account history.

```ts
// tunnel.open  { machine: { type: "machine", id } }  { name: "laptop.florian.destack.computer" }
// tunnel.close { machine: { type: "machine", id } }  { reason: "machine-revoked" | "key-revoked" }
```

## Protocol

A tunnel sends one yamux frame per binary WebSocket message, the relay opens the even streams, and text messages carry only the liveness probe.

```text
byte 0      type: data, window, ping, go-away
byte 1      flags: SYN, ACK, FIN, RST
bytes 2-5   stream
bytes 6-9   window delta, ping value or go-away code
bytes 10-   payload: body bytes, or the request or response head of a SYN or ACK

text "ping" -> text "pong"                                  liveness, answered without waking the object
PUT /tunnel, Authorization: Bearer <token> -> {"name":"laptop.florian.destack.computer","expiresIn":59.8}
```

## Handshake

A machine sends its token in base64url as a second WebSocket subprotocol after `destack.tunnel`, and the relay verifies the token and answers with `destack.tunnel` alone.

```http
GET /tunnel HTTP/1.1
Upgrade: websocket
Sec-WebSocket-Protocol: destack.tunnel, destack.bearer.<token>

HTTP/1.1 101 Switching Protocols
Sec-WebSocket-Protocol: destack.tunnel
```

## Tests

`RelayFixture` enrolls a machine of a test account with a space, and starts relays and tunnels to it, closed on disposal.

```ts
import { RelayFixture } from "@destack/relay/test";

await using relay = await RelayFixture.open(accounts, await RelayFixture.workload(accounts));
await relay.rename("notes-next");
```
