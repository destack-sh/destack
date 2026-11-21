import type { ResourceDeclaration } from "@destack/resource";
import {
    DurableObjectRunner,
    type DurableObjectRunnerBindings,
    type DurableObjectRunnerState,
} from "./runner.ts";
import type { Workload } from "../workload/index.ts";
import { ObjectConnection } from "../client/index.ts";
import { Environment } from "@destack/environment";

/** Host a workload in Durable Objects of the returned class, with its package's resources. */
export function hostWorkload(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
): new (
    state: DurableObjectRunnerState,
    bindings: DurableObjectRunnerBindings,
) => DurableObjectRunner {
    // reach the machine over one connection per URL and secret
    const environment = Environment.system;
    const remote = ObjectConnection.pool(environment);
    const runner = { workload, resources, environment, remote };

    return class extends DurableObjectRunner {
        /** Run the workload in the object's state. */
        constructor(state: DurableObjectRunnerState, bindings: DurableObjectRunnerBindings) {
            super(state, bindings, runner);
        }
    };
}
