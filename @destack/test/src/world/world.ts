import { Simulation, type SimulationOptions } from "@destack/environment/simulation";
import { afterAll, onTestFinished, TestRunner } from "vitest";
import { testHost } from "../host/index.ts";

/** The seed of a test's own simulated world. */
const SEED = 1;

/** The variable that runs every test's world with jitter, exposing real work no world schedules. */
const JITTER_VARIABLE = "DESTACK_SIMULATION_JITTER";

/** The variable that prints a world's span tree and histograms as it ends, when `tree`. */
const TELEMETRY_VARIABLE = "DESTACK_TELEMETRY";

/** The moment a test's own world starts at unless it names another: 2026-10-01T00:00:00Z. */
export const TEST_START = Date.UTC(2026, 9, 1);

/** Open a test's own running world at a start, which its stores and nodes share, until it ends. */
export function testSimulation(
    start = TEST_START,
    options: Omit<SimulationOptions, "seed" | "start"> = {},
): Simulation {
    // stop the world once its test finishes, or once its file's tests did for a world they share
    const { process } = testHost();
    const jitter = options.jitter ?? process.variables[JITTER_VARIABLE] === "1";
    const simulation = new Simulation({ ...options, jitter, seed: SEED, start });
    const isDescribed = process.variables[TELEMETRY_VARIABLE] === "tree";
    const dispose = async () => {
        // describe what the world spent, then stop it
        if (isDescribed) {
            process.stderr.writeSync(`${await simulation.telemetry.describe()}\n`);
        }
        await simulation[Symbol.asyncDispose]();
    };
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
