# @destack/machine

Run a machine: serve the spaces placed on it with their modules (buckets, vaults, connections, messages, observability, audit), route their installations' calls and keep their builds.

## Overview

`@destack/machine` is the host of Destack's spaces, after a kubelet: machines on computers, servers and Cloudflare, serving the spaces placed on them.

A host runs a machine:

- `Machine`: one enrolled machine, serving its placed spaces.
- `Space`: one placed space's installations over its database, events, blobs and sockets.
- `Gateway`: every call into and out of a space's installations.
- `PartitionObject`: a Cloudflare Durable Object running one partition.

A person's computer runs one too:

- `MachineProcess`: the machine's process, started once and reused.
- `Device` and `Launcher`: this computer's people, and the views they open.

Tests run whole worlds:

- `WorldDriver`: a universe and a person's computer in one simulated world.

Answer a request to a space placed on this machine:

```ts
import type { Machine } from "@destack/machine";
import type { Identifier } from "@destack/schema";

/** Answer a request to a space placed on this machine, once the machine is up. */
export async function answer(machine: Machine, spaceId: Identifier<"space">, request: Request) {
    // wait for the machine, then hand the request to the space's gateway
    await machine.health.waitUp();
    const space = await machine.space(spaceId);

    return await space.fetch(request);
}
```

## Reference

### Machine

`new Machine(options)` serves spaces with a machine's identity, storage and runtimes.

| Verb                 | What it does                        |
| -------------------- | ----------------------------------- |
| `enroll(…)`          | registers the machine in an account |
| `space(id)`          | starts or reads a placed space      |
| `serve(request)`     | answers a request to any space here |
| `serveName(request)` | answers a request by its host name  |
| `health.waitUp()`    | waits until the machine serves      |

### Space

`space.fetch(request)` answers probes, else hands a request to the gateway.
The gateway routes each call to the deployment whose `Destack-Version` header matches, as Vercel's skew protection does.

### MachineProcess and Device

| Verb                                  | What it does                                               |
| ------------------------------------- | ---------------------------------------------------------- |
| `MachineProcess.current(environment)` | addresses the current release's machine process            |
| `start(entry)`, `stop()`              | starts the machine through launchd or systemd, or stops it |
| `Device.current(environment)`         | opens this computer's device                               |
| `launcher(login)`, `launch(…)`        | opens a person's launcher, and a view at its origin        |
| `signIn(…)`                           | signs a person in through the machine                      |

## Examples

### Start this computer's machine

`start` reuses a running machine.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { MachineProcess } from "@destack/machine/computer";

/** Start this computer's machine, or reuse the running one. */
export async function startMachine(entry: string) {
    // start the machine of the current release, and read it over loopback
    const process = MachineProcess.current(BunEnvironment.system);
    await process.start(entry);

    return await process.loopback.computer();
}
```

### Open a view as a person

`launch` opens a view at its installation's origin, signed in as the person.

```ts
import { BunEnvironment } from "@destack/environment/bun";
import { Device } from "@destack/machine/device";
import type { Identifier } from "@destack/schema";

/** Open the notes app's home view as a signed-in person. */
export async function openNotes(loginId: Identifier<"login">) {
    // open the person's launcher, and launch the view at its origin
    const device = await Device.current(BunEnvironment.system);
    const launcher = await device.launcher(loginId);
    const address = { space: "home--acme", installation: "notes", view: "home", path: "/" };
    const { url } = await device.launch(launcher, address);

    return url;
}
```

### Run a computer in a test world

`WorldDriver` acts out a world's steps.

```ts
import { WorldDriver } from "@destack/machine/test";

// start a universe and a person's computer, then run a minute
const driver = await WorldDriver.start();
await driver.act({ action: "run", milliseconds: 60_000 });
```

## Comparison

### Kubernetes kubelet

A machine converges the spaces placed on it, as a kubelet does with its pods.

| kubelet  | `Machine`     | Difference                                                       |
| -------- | ------------- | ---------------------------------------------------------------- |
| pods     | placed spaces | A space runs its installations in one process over one database. |
| a node   | a machine     | Also a person's computer, or Cloudflare.                         |
| services | the gateway   | Routes by deployment version, as Vercel's skew protection.       |
