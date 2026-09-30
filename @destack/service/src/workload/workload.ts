import { type ReplicaSource } from "@destack/sync";
import { type ComputeDefinition, declaringModule, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { WorkloadDefinition } from "@destack/package/workload";
import type { ResourceContext } from "@destack/resource/context";
import type { ServiceImplementation } from "../server/index.ts";
import type { RunClient } from "../trigger/index.ts";
import type { Webhook } from "../webhook/index.ts";

/** A declared unit of deployment. */
export interface Workload extends Declaration {
    /** The compute settings of each instance. */
    readonly compute?: ComputeDefinition;
    /** Start one instance and return its services and webhooks. */
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
    /** The source of the copies of the installation's space: its chain, and the global rows it reads. */
    readonly replicas?: { readonly scope: string; readonly source: ReplicaSource };
    /** The cell recording the installation's runs: sent calls and the calls of its triggers' causes. */
    readonly runs: RunClient;
    /** The shutdown signal. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Register cleanup, run in reverse order after draining. */
    defer(dispose: () => void | PromiseLike<void>): void;
}

/** The audit history a workload delivers its events to, in batches of one transaction each. */
export interface AuditHistory {
    /** Store a batch's events once in one transaction. */
    ingest(
        batch: { readonly events: readonly unknown[] },
        options?: { signal?: AbortSignal },
    ): Promise<unknown>;
}

/** The services and webhooks of one workload. */
export interface WorkloadImplementation {
    /** The service implementations. */
    readonly services: readonly ServiceImplementation[];
    /** The webhooks the workload receives. */
    readonly webhooks?: readonly Webhook[];
}
