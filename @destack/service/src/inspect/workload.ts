import { ComputeDefinition } from "@destack/package";
import { WorkloadDefinition } from "@destack/package/manifest";
import type { Workload } from "../workload/index.ts";

/** Describe a workload. */
export function describeWorkload(workload: Workload): WorkloadDefinition {
    return WorkloadDefinition.parse({
        name: workload.name,
        compute: ComputeDefinition.merge(workload.compute),
        ...(workload.placement === undefined ? {} : { placement: workload.placement }),
    });
}
