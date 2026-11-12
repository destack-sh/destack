# @destack/tunnel

Carry HTTP requests over one WebSocket session between a machine and a relay, as a tunnel.

## Overview

A machine opens a tunnel to its residency's relay, which sends the requests for the machine's names through it.

- **TunnelClient**: a machine's tunnel to a relay; `open` dials it, renews its token, and dials again after it ends; `opened` and `close`.
- **Session**: the multiplexed streams over one WebSocket, with flow control and heartbeats, which the relay and the machine share.
- **Renewal**: the name and lapse time the relay grants a tunnel each time its token renews.

```ts
import type { Environment } from "@destack/environment";
import type { PackageId } from "@destack/package";
import type { Identifier } from "@destack/schema";
import { TunnelClient } from "@destack/tunnel";

async (
    machineId: Identifier<"machine">,
    token: (audience: PackageId) => Promise<string>,
    serve: (request: Request) => Promise<Response>,
    environment: Environment,
) => {
    const tunnel = TunnelClient.open({
        url: "https://relay.eu.destack.space/tunnel",
        machineId,
        token,
        fetch: serve,
        renewed: ({ name, lapsesAt }) => console.log(name, lapsesAt),
        changed: (status) => console.log(status), // "open", then "dialing" once it ends
        environment,
    });
    await tunnel.opened();
    await tunnel.close();
};
```

## Examples

### Send a request through a session

`Session.fetch` sends a request as one stream of the session and answers its response.

```ts
import type { Session } from "@destack/tunnel";

async (session: Session) => {
    const response = await session.fetch(new Request("https://notes--work--acme.destack.space/"));
};
```

## Comparison

### HTTP/2

Streams, flow control credit, pings and GOAWAY follow HTTP/2's framing, carried over one WebSocket; two things differ.

- The machine dials out to the relay, so it serves requests from behind any network address translation.
- The tunnel offers a universe token, renews it before it lapses, and dials again after the session ends.
