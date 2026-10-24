import { Simulation, type SimulationOptions } from "@destack/environment/simulation";

/** The seed of a test's own simulated world. */
const SEED = 1;

/** The moment a test's own world starts at unless it names another: 2026-10-01T00:00:00Z. */
export const TEST_START = Date.UTC(2026, 9, 1);

/** Open a test's own simulated world, paused at a start, which its stores and nodes share. */
export function testSimulation(
    start = TEST_START,
    options: Omit<SimulationOptions, "seed" | "start"> = {},
): Simulation {
    return new Simulation({ ...options, seed: SEED, start });
}
