import type { Simulation } from "@destack/environment/simulation";
import { testHost } from "@destack/test/host";
import * as world from "@destack/test/world";

/** The start of every build test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** Make a fresh world on this machine's platform. */
export function testSimulation(): Simulation {
    return world.testSimulation(START, { host: testHost().host });
}
