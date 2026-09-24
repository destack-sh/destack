import { mergeCompute } from "@destack/package";
import { WorkloadDefinition } from "@destack/package/workload";
import type { Workload } from "../workload/index.ts";

/** Describe a declared workload with its checked compute settings. */
export function describeWorkload(workload: Workload): WorkloadDefinition {
    return WorkloadDefinition.parse({
        name: workload.name,
        compute: mergeCompute(workload.compute),
    });
}
