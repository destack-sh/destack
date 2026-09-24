import { type ComputeDefinition, declaringModule, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { WorkloadDefinition } from "@destack/package/workload";
import type { ResourceContext } from "@destack/resource/context";
import type { ServiceImplementation } from "../server/index.ts";
import type { ScheduleImplementation } from "../schedule/index.ts";

/** A declared unit of deployment: code started once per instance and scaled together. */
export interface Workload extends Declaration {
    /** Capacity and lifecycle policy for each instance. */
    readonly compute?: ComputeDefinition;
    /** Start one instance and return the services and schedules it implements. */
    start(context: WorkloadContext): WorkloadImplementation | Promise<WorkloadImplementation>;
}

/** Declare a workload with its compute settings and start function. */
export function defineWorkload(
    definition: WorkloadDefinition & Pick<Workload, "start">,
    module?: ModuleMetadata,
): Workload {
    // stamp the declaring package supplied by the module transform
    const owner = declaringModule(module, "defineWorkload").package;
    const { start, ...fields } = definition;

    return Object.freeze({ ...WorkloadDefinition.parse(fields), start, package: owner });
}

/** Host-supplied resources and cooperative workload lifecycle. */
export interface WorkloadContext {
    /** Prepared resource and service clients selected for this installation. */
    readonly resources: ResourceContext;
    /** Cancellation for observations and background activity. */
    readonly signal: AbortSignal;
    /** Request that the hosting adapter stop this workload. */
    shutdown(): void;
    /** Register cleanup in reverse acquisition order, after service draining. */
    defer(dispose: () => void | PromiseLike<void>): void;
}

/** Services and schedules implemented together by one workload. */
export interface WorkloadImplementation {
    /** Implementations of declared services. */
    readonly services: readonly ServiceImplementation[];
    /** Handlers of declared schedules. */
    readonly schedules?: readonly ScheduleImplementation[];
}
