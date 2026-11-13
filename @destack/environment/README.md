# @destack/environment

Take every effect through one environment: time, randomness and the network, real or simulated.

## Overview

`@destack/environment` is every effect as one object: real environments for Bun, workerd and browsers, and seeded simulations for tests.

Your code takes its effects from an environment:

- `Environment`: the clock, scheduler, randomness, WebCrypto, network, compression and telemetry on every target.
- `Environment<"files" | "process" | …>`: an environment with some host effects.

Targets have real environments:

- `BunEnvironment`, `WorkerdEnvironment` and `BrowserEnvironment`: the system's effects on Bun, Cloudflare Workers and browsers.

Tests use simulations:

- `Simulation`: nodes on one simulated clock, with seeded randomness and a faulty network.
- `World`: a simulation as data: its seed, start and steps.

Write code against an environment, then run it on Bun or in a simulation:

```ts
import type { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { Simulation } from "@destack/environment/simulation";

/** Copy today's note to the backup folder a second later. */
export async function backUpNote(environment: Environment<"files">) {
    const text = await environment.files.readTextFile("/notes/today.md");
    await environment.clock.sleep(1000);
    await environment.files.writeTextFile("/backup/today.md", text);
}

// run it on the system
await backUpNote(BunEnvironment.system);

// run it in a simulation with files in memory
await using simulation = new Simulation({
    seed: 7,
    start: Date.UTC(2026, 10, 12),
    files: "memory",
});
await backUpNote(simulation.environment("host"));
```

## Reference

### Environment

Each effect follows a web or runtime API.

| Effect        | Verbs                                                                          | Prior art                           |
| ------------- | ------------------------------------------------------------------------------ | ----------------------------------- |
| `clock`       | `now`, `sleep`, `timeout`, `setTimeout`, `setInterval`                         | timers and `AbortSignal`            |
| `scheduler`   | `yield`, `run`, `task`, `waitUntil`                                            | the Prioritized Task Scheduling API |
| `random`      | `bytes`, `sequence`                                                            | `crypto.getRandomValues`            |
| `crypto`      | `digest`, `sign`, `verify`, `encrypt`, `decrypt`, …                            | WebCrypto's `SubtleCrypto`          |
| `network`     | `fetch`, `connect`, `websocket`, `resolve`                                     | `fetch`, Cloudflare's `connect`     |
| `compression` | `compress(format)`, `decompress(format)`                                       | `CompressionStream`                 |
| `telemetry`   | `tracers`, `meters`, `loggers`, `propagator`, `context`                        | OpenTelemetry                       |
| `files`       | `readFile`, `writeFile`, `stat`, `readDirectory`, `rename`, `watch`, `lock`, … | Deno's file system API              |
| `process`     | `arguments`, `variables`, `spawn`, `run`, `fork`, `on(signal)`, `exit`         | Deno's `Deno.Command`               |
| `server`      | `http(options)`, `socket(options)`                                             | `Bun.serve`                         |
| `host`        | `os`, `architecture`, `hostname`, `home`                                       | Node's `os`                         |
| `opener`      | `openUrl(url)`                                                                 | the system's URL opener             |
| `workers`     | `start(create)`                                                                | Web Workers                         |

The last six are host effects, requested in the type, such as `Environment<"files" | "process">`.
The network and the server trace each request as OpenTelemetry's HTTP conventions do, and `Environment.with(environment, { telemetry })` moves both onto another telemetry.

### Simulation

`new Simulation({ seed, start, files })` creates a world of nodes, and `environment(node)` gives one node's environment.
Its clock moves only while the test waits, so a seed runs the same every time.

| Verb                                 | What it does                                                 |
| ------------------------------------ | ------------------------------------------------------------ |
| `environment(node)`                  | gives a node's environment, creating the node                |
| `sleep(milliseconds)`, `run(to)`     | moves time, every timer firing on time                       |
| `jump(to)`                           | moves time at once, as a sleeping laptop wakes               |
| `idle()`                             | waits until no work is left                                  |
| `waitUntil(condition, deadline)`     | moves time until a condition holds                           |
| `network.degrade({ drop, delay })`   | drops and delays deliveries                                  |
| `network.partition(left, right)`     | cuts the network between two nodes                           |
| `node(name).crash()`                 | crashes a node, ending its work and breaking its connections |
| `bump(node, by)`                     | jumps a node's clock, as a clock fault does                  |
| `fork(label)`                        | copies the simulation to try another path                    |
| `Simulation.of(world)`, `take(step)` | runs a world from its seed, one step at a time               |

### World

A world is `{ seed, start, steps }`.
Its steps are the shared ones below or a driver's own, and `World.isStep` tells them apart.

| Step                                     | What it does                               |
| ---------------------------------------- | ------------------------------------------ |
| `{ action: "run", milliseconds }`        | runs the world for a while                 |
| `{ action: "jump", milliseconds }`       | moves the clock at once                    |
| `{ action: "partition", nodes }`         | cuts the network between two nodes         |
| `{ action: "heal", nodes }`              | joins two nodes again, or every pair       |
| `{ action: "isolate", node }`, `rejoin`  | cuts a node off from every other, and back |
| `{ action: "crash", node }`              | crashes a node                             |
| `{ action: "degrade", drop, delay }`     | sets the faults every delivery meets       |
| `{ action: "bump", node, milliseconds }` | jumps a node's clock                       |

`World.scrub(world, steps)` keeps a world's first steps, and `World.branch` continues them with new ones.

## Examples

### Append under a lock

`files.lock` takes a shared or exclusive lock other processes respect, released on disposal.
Writing a new file and renaming it over the old one replaces it at once.

```ts
import type { Environment } from "@destack/environment";

/** Append a line to a log that several processes write. */
export async function appendLine(environment: Environment<"files">, path: string, line: string) {
    // hold the log's lock while reading and writing it
    const { files } = environment;
    await using held = await files.lock(`${path}.lock`, "exclusive");
    const text = (await files.exists(path)) ? await files.readTextFile(path) : "";

    // write the next version beside it, then move it into place
    await files.writeTextFile(`${path}.next`, `${text}${line}\n`);
    await files.rename(`${path}.next`, path);
}
```

### Run a command

`process.spawn` starts a command, and `output` waits for its end and reads what it wrote.

```ts
import type { Environment } from "@destack/environment";

/** Read the working tree's status of a Git checkout. */
export async function readStatus(environment: Environment<"process">, directory: string) {
    const child = environment.process.spawn(["git", "status", "--short"], {
        workingDirectory: directory,
    });
    const { code, stdout } = await child.output();

    return code === 0 ? new TextDecoder().decode(stdout) : undefined;
}
```

### Break the network in a simulation

A simulation drops, delays and partitions messages between its nodes.

```ts
import { Simulation } from "@destack/environment/simulation";

// start a world with a laptop and a host
await using simulation = new Simulation({ seed: 7, start: Date.UTC(2026, 10, 12) });
const laptop = simulation.environment("laptop");
const host = simulation.environment("host");

// lose one message in a hundred, and cut the laptop off for five seconds
simulation.network.degrade({ drop: 0.01, delay: { least: 5, most: 80 } });
simulation.network.partition("laptop", "host");
await simulation.sleep(5000);
```

### Replay a world

`Simulation.of` runs a world from its seed, and `take` plays its steps.

```ts
import { World } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";

// describe a minute with the laptop cut off
const world: World = {
    seed: 7,
    start: Date.UTC(2026, 10, 12),
    steps: [
        { action: "partition", nodes: ["laptop", "host"] },
        { action: "run", milliseconds: 60_000 },
    ],
};

// play its shared steps in a simulation
await using simulation = Simulation.of(world);
for (const step of world.steps) {
    if (World.isStep(step)) {
        await simulation.take(step);
    }
}
```

### Take a browser lock

`BrowserEnvironment` adds Web Locks, `BroadcastChannel`, storage, navigation and display.

```ts
import type { BrowserEnvironment } from "@destack/environment/browser";

/** Sync a space in one tab at a time. */
export async function syncOnce(environment: BrowserEnvironment, sync: () => Promise<void>) {
    await environment.locks.request("space", {}, sync);
}
```

## Comparison

### FoundationDB simulation

Simulation basically follows FoundationDB's: every node in one process, on a seeded clock and a faulty network.
Production code takes the same `Environment`, and a run is replayable data.

| FoundationDB                | `@destack/environment`                  | Difference                                                  |
| --------------------------- | --------------------------------------- | ----------------------------------------------------------- |
| `g_network`, `INetwork`     | `Environment`                           | Passed to code instead of a global.                         |
| simulated processes         | `simulation.environment(node)`          | The same.                                                   |
| buggify and fault injection | `degrade`, `partition`, `crash`, `bump` | Steps of a `World`, chosen by the test.                     |
| a seed                      | `World`                                 | The seed plus its steps, which `scrub` and `branch` change. |

### Deno

Host effects follow Deno's APIs, taken as an object instead of the `Deno` global.

| Deno                   | `@destack/environment`         | Difference                                                |
| ---------------------- | ------------------------------ | --------------------------------------------------------- |
| `Deno.readTextFile`, … | `files.readTextFile`, …        | The same, plus `lock` and `tryLock`.                      |
| `Deno.Command`         | `process.spawn`, `process.run` | Takes the command as one array.                           |
| `Deno.serve`           | `server.http`                  | Takes a handler and WebSocket upgrades as in `Bun.serve`. |
| `--allow-read`, …      | `Environment<"files">`         | Asked for in the type, not at run time.                   |
