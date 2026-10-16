import type { ResourceDeclaration } from "@destack/resource";
import {
    DurableObjectRunner,
    type DurableObjectRunnerBindings,
    type DurableObjectRunnerState,
} from "@destack/runtime/cloudflare";
import type { Workload } from "@destack/runtime/workload";
import { spaceRunner } from "../client/index.ts";
import { Clock } from "@destack/environment";

/** Declare the class of the Durable Objects running a workload in a space. */
export function durableObjectSpaceRunner(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
): new (
    state: DurableObjectRunnerState,
    bindings: DurableObjectRunnerBindings,
) => DurableObjectRunner {
    const runner = spaceRunner(workload, resources, Clock.system);

    return class extends DurableObjectRunner {
        /** Run the workload with the stack client of its installation. */
        constructor(state: DurableObjectRunnerState, bindings: DurableObjectRunnerBindings) {
            super(state, bindings, runner);
        }
    };
}
