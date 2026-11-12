# @destack/environment

Take every effect through one environment: time, randomness and the network, real or simulated.

## Overview

Code takes an `Environment` instead of reaching for globals, so the same code runs on the system or in a seeded simulation.

- **Environment**: the effects code takes: `clock`, `scheduler`, `random`, `crypto`, `network`, `compression` and `telemetry` on every target, and `files`, `process`, `server`, `host`, `opener` and `workers` where a target has them, asked for by name.
- **Simulation**: nodes on one simulated clock, seeded randomness and a faulty network; time moves only while the test waits on it, so a seed runs the same every time.
- **World**: a simulation as data, its seed, start and steps, to replay, scrub and branch.

```ts
import type { Environment } from "@destack/environment";
import { BunEnvironment } from "@destack/environment/bun";
import { Simulation } from "@destack/environment/simulation";

async function backup(environment: Environment<"files">) {
    const text = await environment.files.readTextFile("/notes/today.md");
    await environment.clock.sleep(1000);
    await environment.files.writeTextFile("/backup/today.md", text);
}

await backup(BunEnvironment.system);
const start = Date.UTC(2026, 10, 12);
await using simulation = new Simulation({ seed: 7, start, files: "memory" });
await backup(simulation.environment("host"));
```

## Examples

### Use the base effects

Every target has the clock, scheduler, randomness, WebCrypto, network, compression and telemetry.

```ts
import type { Environment } from "@destack/environment";

async (environment: Environment, signal: AbortSignal) => {
    await environment.clock.sleep(30_000, signal);
    const bytes = environment.random.bytes(16);
    const digest = await environment.crypto.digest("SHA-256", bytes);
    const response = await environment.network.fetch(new Request("https://example.com"));
    const socket = environment.network.connect({ hostname: "smtp.example.com", port: 25 });
};
```

### Run a command

`process.spawn` runs a subprocess, and `output` reads its whole output, as Deno's `ChildProcess` does.

```ts
import type { Environment } from "@destack/environment";

async (environment: Environment<"process">) => {
    const child = environment.process.spawn(["git", "status"], { workingDirectory: "/work" });
    const { code, stdout } = await child.output();
};
```

### Break the network in a simulation

A simulation drops, delays and partitions messages between its nodes, and moves time while the test waits.

```ts
import { Simulation } from "@destack/environment/simulation";

await using simulation = new Simulation({ seed: 7, start: Date.UTC(2026, 10, 12) });
const laptop = simulation.environment("laptop");
simulation.network.degrade({ drop: 0.01, delay: { least: 5, most: 80 } });
simulation.network.partition("laptop", "host");
await simulation.sleep(5000);
await simulation.idle();
```

### Replay a world

A `World` lists a simulation's steps, and `Simulation.of` runs it from its seed.

```ts
import { World } from "@destack/environment";
import { Simulation } from "@destack/environment/simulation";

const world: World = {
    seed: 7,
    start: Date.UTC(2026, 10, 12),
    steps: [
        { action: "partition", nodes: ["laptop", "host"] },
        { action: "run", milliseconds: 60_000 },
    ],
};
await using simulation = Simulation.of(world);
for (const step of world.steps) {
    if (World.isStep(step)) {
        await simulation.take(step);
    }
}
const first = World.scrub(world, 1);
```

### Use the browser's effects

`BrowserEnvironment` adds Web Locks, `BroadcastChannel`, storage, navigation and display, each after its web API.

```ts
import type { BrowserEnvironment } from "@destack/environment/browser";

async (environment: BrowserEnvironment) => {
    await environment.locks.request("space", {}, async () => undefined);
    environment.navigation.navigate("/inbox");
};
```

## Comparison

### FoundationDB simulation

Seeded, single-clock simulation of every node and its network follows FoundationDB's simulation testing; three things differ.

- The same `Environment` interface runs on Bun, workerd, the browser and the simulation, so production code takes no test hooks.
- File, process and server effects follow Deno's and Bun's APIs, and the network follows `fetch`, WebSocket and Cloudflare's `connect`.
- A `World` is plain data, so a failing run can be replayed, cut short with `scrub`, or continued another way with `branch`.
