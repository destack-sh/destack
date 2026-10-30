import { Simulation, type SimulationOptions } from "@destack/environment/simulation";
import { afterAll, onTestFinished, TestRunner } from "vitest";

/** The seed of a test's own simulated world. */
const SEED = 1;

/** The moment a test's own world starts at unless it names another: 2026-10-01T00:00:00Z. */
export const TEST_START = Date.UTC(2026, 9, 1);

/** Open a test's own running world at a start, which its stores and nodes share, until it ends. */
export function testSimulation(
    start = TEST_START,
    options: Omit<SimulationOptions, "seed" | "start"> = {},
): Simulation {
    // stop the world once its test finishes, or once its file's tests did for a world they share
    const simulation = new Simulation({ ...options, seed: SEED, start });
    const dispose = () => simulation[Symbol.asyncDispose]();
    if (TestRunner.getCurrentTest() === undefined) {
        afterAll(dispose);
    } else {
        onTestFinished(dispose);
    }

    return simulation;
}
