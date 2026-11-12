# @destack/machine

Run a machine: serve the spaces placed on it with their modules (buckets, vaults, connections, messages, observability, audit), route their installations' calls and keep their builds.

## Overview

A machine enrolls with the universe and serves the spaces placed on it, on a person's computer, on a server or on Cloudflare.

- **Machine**: one machine on a host; `enroll` registers it in an account, `space` starts a placed space, and `serve` and `serveName` answer requests.
- **Space**: one placed space's installations over its `Volume` of database, events, blobs and sockets; `fetch` hands a request to its gateway.
- **Gateway**: every call into and out of a space's installations: routes, origins, mounts, installation calls by deployment, and workloads' calls out with exchanged tokens.
- **Computer**: the Destack process on a person's computer, answering on loopback; `MachineProcess` starts it or reuses the running one.
- **Device**: this computer's device, with its people's sign-ins and a `Launcher` per person that opens their views.
- **PartitionObject**: a Cloudflare Durable Object running one partition: a space's machine or an installation's partition.

```ts
import type { Machine } from "@destack/machine";
import type { Identifier } from "@destack/schema";

async (machine: Machine, spaceId: Identifier<"space">, request: Request) => {
    await machine.health.waitUp();
    const space = await machine.space(spaceId);
    const answer = await space.fetch(request);
    const routed = await machine.serve(request);
};
```

## Examples

### Start this computer's machine

`MachineProcess.start` reuses a running machine, or starts one through launchd or systemd, or launches it.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { MachineProcess } from "@destack/machine/computer";

async (entry: string) => {
    const process = MachineProcess.current(BunEnvironment.system);
    await process.start(entry);
    const computer = await process.loopback.computer();
};
```

### Open a view as a person

A person's `Launcher` addresses a view on its installation's origin, which signs the person in at the universe.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { Device } from "@destack/machine/device";
import type { Identifier } from "@destack/schema";

async (loginId: Identifier<"login">) => {
    const device = await Device.current(BunEnvironment.system);
    const launcher = await device.launcher(loginId);
    const address = { space: "home--acme", installation: "notes", view: "home", path: "/" };
    const { url } = await device.launch(launcher, address);
};
```

### Run a computer in a test world

`WorldDriver` runs a universe and a signed-in person's computer in one simulated world, and acts out a world's steps.

```ts
import { WorldDriver } from "@destack/machine/test";

const driver = await WorldDriver.start();
await driver.act({ action: "run", milliseconds: 60_000 });
```

## Comparison

### Kubernetes kubelet

A machine reports what it runs and converges the spaces placed on it, as a kubelet does with its pods; three things differ.

- A space runs its installations in one process over one database, never one container per package.
- The gateway routes each call by the deployment whose `Destack-Version` matches, as Vercel's skew protection does.
- The same machine runs on a person's computer, a server and Cloudflare, each placement chosen by the universe.
