# @destack/environment

`Clock` and `Network` are the effects code takes, the system's or a `Simulation`'s, which runs nodes on virtual time, seeded randomness and a faulty network.

```ts
import { Clock, Network } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";

await Clock.system.sleep(30_000, signal); // rejects with the signal's reason once it aborts
const listener = Network.system.serve({ port: 0, fetch, websocket }); // Bun.serve
const socket = Network.system.connect(url, ["tunnel"]); // WebSocket, binary as ArrayBuffers
// one seed replays the whole run
const simulation = new Simulation({ seed: 7, start: Date.UTC(2026, 9, 16) }).install(); // Date, timers, crypto, fetch, WebSocket
const host = await simulation.node(
    "host",
    ({ clock, network }, signal) => Host.start({ clock, network, signal }), // its listeners and sockets on the simulated network
);
simulation.network.degrade({ drop: 0.01, delay: { least: 5, most: 80 } }); // a lost message breaks its socket
simulation.network.partition("tab", "host");
host.crash(); // drops its memory, listeners and sockets, keeps its database
await simulation.run({ until: () => run.state === "completed" }); // skips time to each next timer
```
