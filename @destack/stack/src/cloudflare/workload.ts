import type { ResourceDeclaration } from "@destack/resource";
import {
    DurableObjectRunner,
    type DurableObjectRunnerBindings,
    type DurableObjectRunnerState,
} from "@destack/object/cloudflare";
import type { Workload } from "@destack/object/workload";
import { spaceRunner } from "../client/index.ts";
import { Environment, LineExporter } from "@destack/environment";

/** Host a workload in Durable Objects of the returned class, with its package's resources. */
export function hostWorkload(
    workload: Workload,
    resources: Readonly<Record<string, ResourceDeclaration<unknown>>>,
): new (
    state: DurableObjectRunnerState,
    bindings: DurableObjectRunnerBindings,
) => DurableObjectRunner {
    const runner = spaceRunner(workload, resources, Environment.system, (line, severity) =>
        LineExporter.console(line, severity),
    );

    return class extends DurableObjectRunner {
        /** Run the workload with the stack client of its installation. */
        constructor(state: DurableObjectRunnerState, bindings: DurableObjectRunnerBindings) {
            super(state, bindings, runner);
        }
    };
}
