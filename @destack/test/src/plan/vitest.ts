import type { ViteUserConfig } from "vitest/config";
import { BunEnvironment } from "@destack/environment/bun";
import { TestPlan } from "./plan.ts";

/** The variable naming the runtime `destack test` starts the plan in, Bun by default. */
const RUNTIME_VARIABLE = "DESTACK_TEST_RUNTIME";

/** Configure the tests of the package in the working directory by its plan. */
export default async function configuration(): Promise<ViteUserConfig> {
    // plan the package in the working directory, in the runtime this run starts
    const environment = BunEnvironment.system;
    const plan = await TestPlan.of(environment.process.workingDirectory, environment.files);
    const runtime = environment.process.variables[RUNTIME_VARIABLE] === "node" ? "node" : "bun";

    return plan.configuration(environment, runtime);
}
