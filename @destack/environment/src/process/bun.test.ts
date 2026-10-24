import { join } from "node:path";
import { expect, test } from "@destack/test";
import { Simulation } from "../simulation/index.ts";
import { BunProcess } from "./bun.ts";

/** The start of the simulated clock. */
const START = Date.UTC(2026, 9, 24);

/** The child that echoes each message from its parent until the parent disconnects. */
const ECHO = `
import { BunProcess } from ${JSON.stringify(join(import.meta.dirname, "bun.ts"))};

const { parent } = new BunProcess();
parent.on((message) => parent.send({ echo: message }));
await parent.disconnected;
`;

test("echo a message through a connected child, and settle both ends once the parent disconnects", async () => {
    // start the echoing child connected
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("channel-");
    const script = join(directory, "echo.ts");
    await files.writeTextFile(script, ECHO);
    const host = new BunProcess();
    const child = host.fork([host.executable, script]);

    // send a message and read the echo
    const { promise: echoed, resolve } = Promise.withResolvers<unknown>();
    using _subscription = child.channel.on(resolve);
    child.channel.send({ ping: 1 });
    const echo = await echoed;

    // disconnect and wait for the child to end
    child.channel.disconnect();
    await child.channel.disconnected;
    const code = await child.exited;

    expect({ echo, code }).toEqual({ echo: { echo: { ping: 1 } }, code: 0 });
});
