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

Every target has the clock, randomness, WebCrypto, network, compression, telemetry, locale and time zone.

```ts
await environment.clock.sleep(30_000, signal); // rejects with the signal's reason once it aborts
environment.random.bytes(16);
await environment.crypto.digest("SHA-256", bytes); // crypto.subtle's operations
await environment.network.fetch(new Request("https://notes.test/today"));
environment.network.websocket(url, ["tunnel"]); // a WebSocket, binary as ArrayBuffers
environment.network.connect({ hostname: "smtp.test", port: 25 }); // a TCP socket, as Cloudflare's connect()
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
await environment.server.tcp({ hostname: "127.0.0.1", accept });
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
await environment.clipboard.writeText(text);
environment.display.media("(prefers-color-scheme: dark)"); // matchMedia
// each node a tab of one origin, sharing its locks and broadcast channels
const tab = new BrowserSimulation(simulation).environment("tab");
```

## Simulation

A `Simulation` gives each node an environment on one clock, randomness and network, and moves time once its work settles.

```ts
await using simulation = new Simulation({
    seed: 7,
    start: Date.UTC(2026, 9, 24),
    timeZone: "Europe/Vienna",
});
const host = await simulation.add("host", (environment, signal) => Host.start(environment, signal));
simulation.network.degrade({ drop: 0.01, delay: { least: 5, most: 80 } }); // a lost message breaks its socket
simulation.network.partition("tab", "host");
host.crash(); // drops its memory, listeners and sockets, keeps its files and database
await simulation.run({ until: () => run.state === "completed" }); // skips time to each next timer
await simulation.jump(simulation.clock.now() + 30 * DAY); // a 30-day sleep: each due wait wakes once
simulation.telemetry.spans; // every span the nodes ended
// work that never settles fails the run as busy, and a condition no timer meets as idle
// a child's output() holds time as host work, and simulated breaks fail with ECONNRESET as Bun's do
new Simulation({ seed: 7, start, isTracing: true }).trace; // every start, crash, timer and delivery, off by default
```
