import { expect, test } from "@destack/test";
import { join } from "node:path";
import { BunEnvironment } from "../bun.ts";
import { BunNetwork } from "./bun.ts";
import { Simulation } from "../simulation/index.ts";

/** The time the simulation starts at. */
const START = Date.UTC(2026, 9, 24);

test("fetch a listener on a Unix socket through the system network, and refuse a Unix socket in a simulation", async () => {
    // serve a greeting on a Unix socket in a temporary directory
    const { files, server } = BunEnvironment.system;
    const directory = await files.makeTemporaryDirectory("destack-unix-");
    const unix = join(directory, "listener.sock");
    const listener = server.http({ unix, fetch: () => new Response("hello") });

    // fetch it at any host, then refuse the same request in a simulation
    try {
        const response = await BunNetwork.fetch(new Request("http://localhost/"), { unix });
        await using simulation = new Simulation({ seed: 1, start: START });
        const refusal = await simulation
            .environment("node")
            .network.fetch(new Request("http://localhost/"), { unix })
            .catch((error: unknown) => error);
        expect([await response.text(), refusal]).toEqual([
            "hello",
            new TypeError("a simulated request needs a host name and port"),
        ]);
    } finally {
        await listener.stop(true);
        await files.remove(directory, { recursive: true, force: true });
    }
});
