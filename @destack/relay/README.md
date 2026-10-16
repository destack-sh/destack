# @destack/relay

`@destack/relay` forwards requests for a space's or a machine's name to the machine serving it, through a tunnel the machine opens to the relay or to the machine's published endpoint.

```ts
await fetch("https://pages--feature-x---personal--ada.destack.space/");
const tunnel = TunnelClient.open({ url, token, fetch, renewed, environment });
```

## Server

`implementRelay` runs a residency's relay installation: the root routes names, and each machine's tunnel lives in a partition of its own.

```ts
import { implementRelay } from "@destack/relay/server";

const served = implementRelay({
    origin: "https://relay.destack.space",
    database,
    events,
    callKey,
    accounts,
    server,
    hibernation,
    issuer, // the universe's issuer, verified by the identity keys the relay replicates
    fetch: (request) => environment.network.fetch(request),
    clock: environment.clock,
});
// the root without server.partition, a machine's tunnel partition with it
const answer =
    served.kind === "root"
        ? await served.relay.fetch(request) // a name's request, or a machine's handshake
        : await served.tunnel.fetch(request); // a request through the machine's tunnel
```

## Installing

A residency home's stack installs the relay from its package handle.

```ts
import relayPackage from "@destack/relay/package";

defineStack({ installations: { relay: install(relayPackage, {}, { alias: "relay" }) } });
```

## Tunnels

`TunnelClient.open` opens a machine's tunnel, renews its token, and dials again after the tunnel ends; at each renewal the machine publishes its endpoint.

```ts
import { TunnelClient } from "@destack/relay/tunnel";

const tunnel = TunnelClient.open({
    url: "https://relay.destack.space/tunnel",
    token: () => identity.token(relayPackage.id, issuer, fetch),
    fetch: (request) => gateway.relay(request),
    renewed: ({ name, lapsesAt }) =>
        machine.publish({ url: `https://${name}`, relay: { url: relayOrigin, lapsesAt } }),
    environment,
});
await tunnel.opened();
```

## Tests

`RelayHost` serves a relay installation in process, and `RelayFixture` enrolls a test machine with a space and starts relays and tunnels to it, all inside one simulation.

```ts
const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 16) }).install();
await using fixture = await RelayFixture.open(simulation, accounts, await accounts.install());
const relay = await fixture.relay();
await fixture.tunnel(relay);
await fixture.until(async () => (await fixture.endpoint()) !== null); // runs virtual time
```
