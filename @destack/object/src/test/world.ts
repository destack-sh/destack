import type { Environment } from "@destack/environment";
import { SimulatedScheduler } from "@destack/environment/simulation";
import { present } from "@destack/schema";
import { TestTelemetry } from "@destack/telemetry/test";

export { TEST_START, testSimulation } from "@destack/test/world";

/** Read the one reader of the telemetry of the world a node runs in. */
export function worldTelemetry(environment: Environment): TestTelemetry {
    const world = present(SimulatedScheduler.of(environment).world, "the node's world");

    return TestTelemetry.of(world.telemetry);
}
