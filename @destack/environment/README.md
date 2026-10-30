# @destack/environment

An `Environment` is every effect code takes: the system's at an entry point, or a `Simulation`'s, which runs nodes on simulated time, seeded randomness and a faulty network.

```ts
import { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { Simulation } from "@destack/environment/simulation";

// the base every target has, and the capabilities code asks for by name
function backup(environment: Environment<"files" | "process">) {}
backup(BunEnvironment.system); // at the process's entry point only
const world = new Simulation({ seed: 7, start: Date.UTC(2026, 9, 24) }).environment("host");
backup(world); // the same code in a seeded world
```

## Base

Every target has the clock, scheduler, randomness, WebCrypto, network, compression, telemetry, locale and time zone.

```ts
await environment.clock.sleep(30_000, signal); // rejects with the signal's reason once it aborts
await environment.scheduler.yield(); // the web's scheduler.yield()
await environment.scheduler.track(statement, "peer"); // work a world counts until it settles: "world", "host" or "peer"
environment.random.bytes(16);
await environment.crypto.digest("SHA-256", bytes); // crypto.subtle's operations
await environment.network.fetch(new Request("https://notes.test/today"));
environment.network.websocket(url, ["tunnel"]); // a WebSocket, binary as ArrayBuffers
environment.network.connect({ hostname: "smtp.test", port: 25 }); // a TCP socket, as Cloudflare's connect()
environment.network.connect({ unix: "/run/destack.sock" }); // a Unix socket, as Bun.connect({ unix })
body.pipeThrough(environment.compression.compress("gzip")); // CompressionStream
environment.telemetry.tracers.getTracer("notes"); // OpenTelemetry's providers, stamped on telemetry.clock
new TraceIdGenerator(environment); // OpenTelemetry's IdGenerator, trace ids starting with their millisecond
at.toLocaleString(environment.locale, { timeZone: environment.timeZone });
```

## Capabilities

`files`, `process`, `server`, `host` and `workers` exist on some targets, and code asks for each by name.

```ts
// Deno's file system API, failing with NOT_FOUND, ALREADY_EXISTS, PERMISSION_DENIED or FILE_SYSTEM_FAILED
await environment.files.readTextFile(join(home, "notes.md"));
await using lock = await environment.files.lock(join(home, "notes.lock"), "exclusive");
// Bun's Subprocess, its whole output read as Deno's ChildProcess.output
const { code, stdout } = await environment.process
    .spawn(["git", "status"], { workingDirectory: home })
    .output();
environment.process.variables["HOME"];
// Bun.serve and Bun.listen
environment.server.http({ hostname: "127.0.0.1", port: 0, fetch });
await environment.server.socket({ hostname: "127.0.0.1", accept });
environment.host.os; // "darwin" | "linux" | "win32"
environment.workers.start(new URL("./worker.ts", import.meta.url), { name: "database" });
```

## Browser

`@destack/environment/browser` adds the browser's effects, each after its web API.

```ts
import { BrowserEnvironment } from "@destack/environment/browser";
import { BrowserSimulation } from "@destack/environment/browser/simulation";

// the Web Locks API, one origin's locks across its tabs and workers
await environment.locks.request("destack:space", { signal }, async (lock) => lead(lock));
// BroadcastChannel
const port = environment.broadcast.open("destack:space");
using _subscription = port.on((message) => apply(message));
await environment.storage.persist(); // the Storage Standard's persist(), beside localStorage's getItem and setItem
environment.navigation.navigate("/inbox"); // the Navigation API
environment.display.media("(prefers-color-scheme: dark)"); // matchMedia
// each node a tab of one origin, sharing its locks and broadcast channels
const tab = new BrowserSimulation(simulation).environment("tab");
```

## Simulation

A `Simulation` runs nodes on one clock, scheduler, randomness and network, and moves time only once no tracked work runs.

```ts
await using simulation = new Simulation({
    seed: 7,
    start: Date.UTC(2026, 9, 24),
    files: "memory", // the nodes' files in memory, "host" by default
    skew: { offset: 500, drift: 100 }, // each node's clock off by up to 500 ms, drifting up to 100 ppm
    isTracing: true, // each event a log record and each tracked work a span, on simulated time
});
const host = simulation.node("host");
await host.start((environment, signal) => Host.start(environment, signal));
simulation.network.degrade({ drop: 0.01, delay: { least: 5, most: 80 } }); // a lost message breaks its socket
simulation.network.partition("tab", "host");
simulation.network.publish("host", { hostname: "127.0.0.1", port: 0, fetch }); // url in the world, hostUrl on the host
simulation.network.bridge("lab"); // http://<origin>.localhost:<port> reaches https://<origin> in the world
simulation.workers.define(new URL("./db.worker.ts", import.meta.url), serve); // a worker body, run as a node
host.crash(); // drops its memory, listeners, sockets, children, workers and locks, keeps its files
await simulation.idle(); // the next point with nothing left before the next timer
await simulation.until(() => run.state === "completed", deadline); // moving time to each next timer
await simulation.jump(simulation.clock.now() + 30 * DAY); // a 30-day sleep: each due wait wakes once
simulation.scheduler.pending(); // the work in flight: who finishes it, running or settled, and where it began
simulation.fingerprint; // equal across runs of one seed and the same work
// a stalled host work fails the run after 10 s, a stalled peer after 2 min, busy work without time as a livelock
```

A `World` is a simulation as data: its seed, start and steps, the shared ones taken by `Simulation.take`.

```ts
import { World } from "@destack/environment";

const world: World<AppStep> = {
    seed: 7,
    start,
    steps: [
        { action: "partition", nodes: ["laptop", "home"] },
        { action: "run", milliseconds: 60_000 },
        { action: "open", computer: "laptop" }, // a driver's own step
    ],
};
await using simulation = Simulation.of(world);
for (const step of world.steps) {
    await (World.isStep(step) ? simulation.take(step) : driver.act(step));
}
World.scrub(world, 2); // its first two steps
World.branch(world, 1, [{ action: "crash", node: "home" }]); // its first step, then new ones
```
