# @destack/relay

`@destack/relay` is Cloudflare Tunnel for Destack names: each machine dials out over a WebSocket carrying yamux-shaped frames, and the relay forwards requests for a space's or a machine's name through that tunnel or to the machine's published endpoint.

```ts
await fetch("https://pages--feature-x---personal--ada.destack.space/"); // a tunnel's public hostname
const tunnel = TunnelClient.open({ url, token, fetch, name, report }); // cloudflared tunnel run
const relay = implementRelay(options); // Cloudflare's edge
WINDOW_BYTES; // 256 KiB of credit per stream, as yamux's window
```

## Service

`implementRelay` runs a residency's relay installation, routing names at its root and keeping each machine's tunnel in a partition of its own.

```ts
import { implementRelay } from "@destack/relay/server";

const relay = implementRelay({
    origin: "https://relay.destack.space",
    database, // the residency home's main database
    accounts, // the account service, followed for accounts, machines and the directory
    installation, // its id, scope, partition, and how it reaches itself
    hibernation, // the WebSockets its host keeps open
    tokens,
    callKey,
    clock,
    report,
});
await relay.relay.route(request); // a name's request, or a machine's WebSocket upgrade
await relay.tunnels.fetch(machineId, request); // inside the machine's tunnel partition
```

A residency home's stack installs it from its package handle.

```ts
import relayPackage from "@destack/relay/package";

defineStack({ installations: { relay: install(relayPackage, {}, { alias: "relay" }) } });
```

## Tunnels

`TunnelClient.open` keeps a machine's tunnel open, renews its token and dials again with backoff whenever the tunnel ends.

```ts
import { TunnelClient } from "@destack/relay/tunnel";

const tunnel = TunnelClient.open({
    url: "https://relay.destack.space/tunnel",
    token: async () => (await identity.token(RELAY_PACKAGE.id, accounts, fetch)).accessToken,
    fetch: (request) => gateway.relay(request),
    name: (name) => publish(`https://${name}`),
    heartbeat: 15_000,
    report,
});
await tunnel.opened();
```

## Tests

`RelayHost` serves a relay installation in process as a machine hosts it, and `RelayFixture` enrolls a machine of a test account with a space and starts relays and tunnels to it.

```ts
await using fixture = await RelayFixture.open(accounts, await accounts.relay());
const relay = await fixture.relay(await freePort(), { destinations: new DestinationCache() });
await fixture.tunnel(relay);
```
