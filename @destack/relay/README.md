# @destack/relay

Relay HTTP requests for Destack names to the machines serving them.

## Overview

`@destack/relay` is the edge of Destack's names, after Cloudflare Tunnel: name resolution, machine tunnels, and request forwarding per residency.

A residency's home installs it with three settings:

- `origin`: the relay's own origin, where machines open their tunnels.
- `residency`: the residency it serves.
- `issuer`: the issuer of machines' tunnel tokens.

Routing:

- `Relay`: resolves names, accepts tunnels and forwards requests.
- `TunnelToken`: verifies a machine's token.
- `TunnelStanding`: whether a machine may keep its tunnel.
- `RelayError`: the reasons a name or tunnel is refused, checked with `RelayError.is`.

Tests:

- `TestTunnelMachine`: a simulated machine with relays and tunnels.

Install the relay in a residency's home, and route the spaces' and machines' names to it:

```ts
import relayPackage from "@destack/relay/package";
import { defineStack } from "@destack/stack";
import { install } from "@destack/stack/declare";

// install the relay, and route both domains to it
export const residency = defineStack({
    types: [],
    installations: { relay: install(relayPackage, {}, { name: "relay" }) },
    routes: {
        spaces: {
            hostname: "*.destack.space",
            path: "/",
            match: "prefix",
            installation: "relay",
            rules: [{ entry: { kind: "mount", mount: "names" } }],
        },
        machines: {
            hostname: "*.destack.computer",
            path: "/",
            match: "prefix",
            installation: "relay",
            rules: [{ entry: { kind: "mount", mount: "names" } }],
        },
    },
});
```

## Reference

### Names

The relay reads a request's host as one of these names, `<handle>` being the account's handle.
A tunnelled machine gets requests through its tunnel, and a published one directly with a `Forwarded` header.

| Name                                                                                                            | Routes to                                                                                                                                    |
| --------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `<installation>--<space>--<handle>.destack.space`, such as `notes--work--acme.destack.space`                    | the machine the space is placed at                                                                                                           |
| `<installation>--<branch>---<space>--<handle>.destack.space`, such as `notes--draft---work--acme.destack.space` | the machine the space's branch is placed at                                                                                                  |
| `<digest>---<space>--<handle>.destack.space`                                                                    | the same, for a branch label over 63 characters, the digest the first 10 hexadecimal digits of the FNV-1a hash of `<installation>--<branch>` |
| `s3--<space identifier>.destack.space`                                                                          | the machine serving the space's buckets over S3                                                                                              |
| `<machine>--<handle>.destack.computer`, such as `laptop--florian.destack.computer`                              | the machine itself                                                                                                                           |

### Relay

A `Relay` answers `/tunnel` at its origin, and forwards every other request.
Each tunnel runs in its own partition, opened with a ten-minute token.

| Verb                   | What it does                                                                 |
| ---------------------- | ---------------------------------------------------------------------------- |
| `fetch(request)`       | answers the tunnel's metadata, opens a machine's tunnel, or relays a request |
| `handshake(request)`   | accepts a machine's WebSocket with a token of a standing, resident machine   |
| `standing(machineId)`  | reads whether a machine stands, and its name                                 |
| `machines(accountId)`  | lists an account's live machines                                             |
| `tunnelled()`          | lists the machines whose endpoints lead to tunnels here                      |
| `reconcile(machineId)` | renames or closes a machine's tunnel as its replica changed                  |

## Examples

### Test a tunnel

`TestTunnelMachine` runs a machine with relays and tunnels in a simulated universe.

```ts
import { TestTunnelMachine } from "@destack/relay/test";
import { expect, test } from "@destack/test";

test("reach a space's app through the computer's tunnel", async () => {
    // start a relay, and open the machine's tunnel
    await using computer = await TestTunnelMachine.open();
    const relay = await computer.relay();
    await computer.tunnel(relay);

    // request the notes app's root through the relay
    const answer = await computer.request(relay, computer.notes, "/");

    expect(typeof answer).toBe("string");
});
```

## Comparison

### Cloudflare Tunnel

Machines open outbound tunnels, as `cloudflared` does to Cloudflare's edge.

| Cloudflare Tunnel       | `@destack/relay`                    | Difference                                                                      |
| ----------------------- | ----------------------------------- | ------------------------------------------------------------------------------- |
| Cloudflare's edge       | one `Relay` per residency           | Requests for a residency's accounts stay in the residency.                      |
| a tunnel token          | the machine's universe token        | Only an account's standing machines serve its names, renewed every ten minutes. |
| public hostnames in DNS | names from the universe's directory | A space's name follows its placement to whichever machine serves it.            |
| none                    | published endpoints                 | A cloud machine gets its requests directly, without a tunnel.                   |
