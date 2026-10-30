import { expect, test } from "@destack/test";
import { Simulation } from "../simulation/simulation.ts";
import { World, type WorldStep } from "./world.ts";

/** The start of the worlds, in UTC epoch milliseconds. */
const START = Date.UTC(2026, 9, 1);

/** A driver's step: fetch a path from one node to another. */
interface FetchStep {
    /** The action. */
    readonly action: "fetch";
    /** The path. */
    readonly path: string;
}

/** A world of shared steps around two fetches, the second across a partition. */
const WORLD: World<FetchStep> = {
    seed: 7,
    start: START,
    steps: [
        { action: "degrade", delay: { least: 5, most: 15 } },
        { action: "fetch", path: "/first" },
        { action: "run", milliseconds: 60_000 },
        { action: "partition", nodes: ["client", "server"] },
        { action: "fetch", path: "/second" },
        { action: "heal" },
        { action: "jump", milliseconds: 3_600_000 },
    ],
};

/** Play a world, its shared steps by its simulation and its fetches by hand. */
async function play(world: World<FetchStep>) {
    // serve each path back
    await using simulation = Simulation.of(world);
    simulation.network.server("server").http({
        hostname: "server.test",
        port: 443,
        fetch: (request) => new Response(new URL(request.url).pathname),
    });
    const { network } = simulation.environment("client");

    // take each step, noting each fetch's answer
    const answers: string[] = [];
    for (const step of world.steps) {
        // take a shared step
        if (World.isStep(step)) {
            await simulation.take(step);
        }
        // fetch by hand
        else {
            const answer = await network
                .fetch(new Request(`https://server.test${step.path}`))
                .then((response) => response.text())
                .catch((error: Error) => error.message);
            answers.push(answer);
        }
    }

    return { answers, at: simulation.clock.now() - START, fingerprint: simulation.fingerprint };
}

test("play a world's shared steps by its simulation and replay it to the same fingerprint", async () => {
    const [first, again] = [await play(WORLD), await play(WORLD)];

    // answer across the latency, refuse across the partition, and end after the run, the jump and both trips
    expect({ answers: first.answers, at: first.at, replayed: again.fingerprint }).toEqual({
        answers: ["/first", "fetch failed: client cannot reach server"],
        at: 3_660_015,
        replayed: first.fingerprint,
    });
});

test("scrub a world to its first steps and branch it into new ones", async () => {
    // scrub to the first three steps, then branch after them without the partition
    const scrubbed = World.scrub(WORLD, 3);
    const steps: readonly (WorldStep | FetchStep)[] = [{ action: "fetch", path: "/branch" }];
    const branched = World.branch(WORLD, 3, steps);

    expect({
        scrubbed: scrubbed.steps.length,
        played: (await play(branched)).answers,
        isShared: WORLD.steps.map((step) => World.isStep(step)),
    }).toEqual({
        scrubbed: 3,
        played: ["/first", "/branch"],
        isShared: [true, false, true, true, false, true, true],
    });
});
