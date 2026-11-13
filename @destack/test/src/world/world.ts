import { Simulation, type SimulationOptions, WorldOptions } from "@destack/environment/simulation";
import { afterAll, onTestFinished, TestRunner } from "vitest";
import { testHost } from "../host/index.ts";

/** The seed of a test's own simulated world. */
const SEED = 1;

/** The moment a test's own world starts at unless it names another: 2026-10-01T00:00:00Z. */
export const TEST_START = Date.UTC(2026, 9, 1);

/** Read the options the test run gives every world: jitter, and the readback it asks for. */
export function testWorldOptions(): WorldOptions {
    return WorldOptions.of(testHost().process);
}

/** Open a test's own running world at a start, which its stores and nodes share, until it ends. */
export function testSimulation(
    start = TEST_START,
    options: Omit<SimulationOptions, "seed" | "start" | "world"> = {},
): Simulation {
    // stop the world once its test finishes, or once its file's tests did for a world they share
    const simulation = new Simulation({
        ...options,
        world: testWorldOptions(),
        seed: SEED,
        start,
    });
    const dispose = () => simulation[Symbol.asyncDispose]();
    const current = TestRunner.getCurrentTest();
    if (current === undefined) {
        afterAll(dispose);
    }
    // leave a concurrent test's world to its own disposal, as Vitest's current test may be a neighbour
    else if (current.concurrent !== true) {
        onTestFinished(dispose);
    }

    return simulation;
}
