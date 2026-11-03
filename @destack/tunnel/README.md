# @destack/tunnel

`TunnelClient.open` opens a machine's tunnel to a relay, renews its token, and dials again after the tunnel ends; the relay's server and the machine share its session protocol.

```ts
import { TunnelClient } from "@destack/tunnel";

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
await tunnel.close();
```
