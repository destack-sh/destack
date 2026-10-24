# @destack/relay

`@destack/relay` forwards requests for a space's or a machine's name to the machine serving it, through a tunnel the machine opens to the relay or to the machine's published endpoint.

```ts
await fetch("https://pages--feature-x---personal--ada.destack.space/");
const tunnel = TunnelClient.open({ url, machineId, token, fetch, renewed, environment });
```

## Server

`relayWorkload` runs a residency's relay installation as a workload with no durable rows: its root routes names over an in-memory replica of the universe's directory, and each machine's tunnel runs in a partition of its own over the sockets its host hibernates.

```ts
import { relayWorkload } from "@destack/relay/server";

// a host starts the root without a partition, and each machine's tunnel in the partition it names
const runner = await WorkloadRunner.start(
    { workload: relayWorkload, resources: {}, publisher, socketHost, ephemeral, environment },
    { ...start, partition: `tunnel-${Identifier.uuid(machineId)}` },
    build,
    startTelemetry,
    report,
);
// each socket keeps { machineId, lapsesAt, kid } beside it; tunnel.open and tunnel.close are telemetry events
```

## Settings

The installation's `origin` setting is where machines open their tunnels, and its `issuer` setting is whose tokens they offer.

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
    machineId: machine.id,
    token: () => identity.token(relayPackage.id, issuer, fetch),
    fetch: (request) => gateway.relay(request),
    renewed: ({ name, lapsesAt }) =>
        machine.publish({ url: `https://${name}`, relay: { url: relayOrigin, lapsesAt } }),
    environment,
});
await tunnel.opened();
```

## Tests

`TestRelay` runs the relay's workload in process behind its machine's front door, its universe rows followed as its residency's cloud machine follows them, and `TestTunnelMachine` enrolls a person's computer serving a space in a simulated universe and starts relays and tunnels to it.

```ts
await using computer = await TestTunnelMachine.open(); // the acme account in Europe
const relay = await computer.relay();
await computer.tunnel(relay);
await computer.request(relay, computer.notes, "/"); // "hello GET /"
computer.tunnels(); // [["tunnel.open", { machine }]]
```
