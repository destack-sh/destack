import { type ComputeDefinition, declaringModule, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { WorkloadDefinition } from "@destack/package/workload";
import type { ResourceContext } from "@destack/resource/context";
import type { AccessRelay } from "@destack/access";
import type { ServiceImplementation } from "../server/index.ts";
import type { TriggerImplementation } from "../trigger/index.ts";

/** A declared unit of deployment. */
export interface Workload extends Declaration {
    /** The compute settings of each instance. */
    readonly compute?: ComputeDefinition;
    /** Start one instance and return its services and triggers. */
    start(context: WorkloadContext): WorkloadImplementation | Promise<WorkloadImplementation>;
}

/** Declare a workload. */
export function defineWorkload(
    definition: WorkloadDefinition & Pick<Workload, "start">,
    module?: ModuleMetadata,
): Workload {
    // stamp the declaring package
    const owner = declaringModule(module, "defineWorkload").package;
    const { start, ...fields } = definition;

    return Object.freeze({ ...WorkloadDefinition.parse(fields), start, package: owner });
}

/** The context of a starting workload. */
export interface WorkloadContext {
    /** The installation's resource and service clients. */
    readonly resources: ResourceContext;
    /** The audit history the workload's outboxes deliver to. */
    readonly history: AuditHistory;
    /** The relay of the access of the installation's space and its containing scopes. */
    readonly access?: AccessRelay;
    /** The shutdown signal. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Register cleanup, run in reverse order after draining. */
    defer(dispose: () => void | PromiseLike<void>): void;
}

/** The audit history a workload delivers its events to, in batches of one transaction each. */
export interface AuditHistory {
    /** Store a batch's events in one transaction, each once. */
    ingest(
        batch: { readonly events: readonly unknown[] },
        options?: { signal?: AbortSignal },
    ): Promise<unknown>;
}

/** The services and triggers of one workload. */
export interface WorkloadImplementation {
    /** The service implementations. */
    readonly services: readonly ServiceImplementation[];
    /** The trigger handlers. */
    readonly triggers?: readonly TriggerImplementation[];
}
