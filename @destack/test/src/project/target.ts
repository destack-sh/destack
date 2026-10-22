import { inject } from "vitest";
import type { TestProject } from "vitest/node";

/** The values a package's tests run against by kind, each combination one project. */
export type TestMatrix = Readonly<Record<string, readonly [string, ...string[]]>>;

/** The values a run's setup provides to its tests. */
declare module "vitest" {
    /** The values a run's setup provides to its tests. */
    export interface ProvidedContext {
        /** The value of each target kind the test's project runs against, such as its database. */
        targets?: Readonly<Record<string, string>>;
    }
}

/** Read the target of a kind a test or a setup's project runs against, such as its database. */
export function testTarget(
    kind: string,
    project?: Pick<TestProject, "getProvidedContext">,
): string {
    // refuse a kind the test configuration declares no target of
    const provided =
        project === undefined ? inject("targets") : project.getProvidedContext().targets;
    const value = provided?.[kind];
    if (value === undefined) {
        throw new TypeError(
            `the test runs against no ${kind}: declare it in the configuration's targets`,
        );
    }

    return value;
}
