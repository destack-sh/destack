# @destack/relay

Relay HTTP requests for Destack names to the machines serving them.

## Overview

Each residency's relay answers the names of the spaces and machines in its accounts, and forwards each request through the tunnel its machine opened.

- **Name**: a host such as `notes--work--acme.destack.space`, made of an installation, a space and an account.
- **Tunnel**: a socket a machine opens to its residency's relay, with the universe's token, which the relay forwards requests through.
- **Relay**: one residency's relay installation, routing names over a replica of the universe's directory; its `origin`, `residency` and `issuer` settings set where tunnels open and whose tokens they offer.

```ts
const response = await fetch("https://notes--work--acme.destack.space/");
```

## Examples

### Install a relay

A residency's home installs the relay, with its `origin`, `residency` and `issuer` settings.

```sh
destack install @destack/relay --name relay
```

### Test a tunnel

`TestTunnelMachine` enrolls a computer serving a space in a simulated universe, and starts relays and tunnels to it.

```ts
import { TestTunnelMachine } from "@destack/relay/test";

await using computer = await TestTunnelMachine.open();
const relay = await computer.relay();
await computer.tunnel(relay);
const answer = await computer.request(relay, computer.notes, "/");
```

## Comparison

### Cloudflare Tunnel

Machines open outbound tunnels that the relay forwards public requests through, as Cloudflare Tunnel does; three things differ.

- A tunnel authenticates with the machine's universe token, so only the account's machines serve its names.
- One relay per residency serves its accounts, so requests stay in the residency.
- A machine with a published endpoint gets requests directly, without a tunnel.
