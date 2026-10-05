import { type Publisher } from "@destack/sync";
import { type CapabilityName, type ComputeDefinition, ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { type Tier, WorkloadDefinition } from "@destack/package/workload";
import type { ResourceContext } from "@destack/resource/context";
import type { BuildReader } from "@destack/package/manifest";
import type { LocaleTag } from "@destack/locale";
import type { ServiceImplementation } from "../server/index.ts";
import type { RunClient } from "../trigger/index.ts";
import type { Trigger } from "../trigger/index.ts";
import { type CallKey } from "../request/index.ts";

/** A declared unit of deployment. */
export interface Workload extends Declaration {
    /** The compute settings of each instance. */
    readonly compute?: ComputeDefinition;
    /** The package's capabilities the workload uses, every one when absent. */
    readonly capabilities?: readonly CapabilityName[];
    /** The placements a universe chooses from for the workload, absent for a workload spaces install. */
    readonly placement?: readonly Tier[];
    /** Start one instance and return its services and webhooks. */
    start(context: WorkloadContext): WorkloadImplementation | Promise<WorkloadImplementation>;
}

/** Declare a workload. */
export function defineWorkload(
    definition: WorkloadDefinition & Pick<Workload, "start">,
    module?: ModuleMetadata,
): Workload {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineWorkload").package;
    const { start, ...fields } = definition;

    return Object.freeze({ ...WorkloadDefinition.parse(fields), start, package: owner });
}

/** The context of a starting workload. */
export interface WorkloadContext {
    /** The installation's resource and service clients. */
    readonly resources: ResourceContext;
    /** The audit history the workload's outboxes deliver to, absent for a workload journaling only in its own database. */
    readonly history?: AuditHistory;
    /** The installation the workload runs as, absent outside a space. */
    readonly installation?: InstallationContext;
    /** The cell recording the installation's runs: sent calls and the calls of its triggers' causes, absent outside a space. */
    readonly runs?: RunClient;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** The shutdown signal. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Report a failure of the workload's background work to its host. */
    report(error: unknown): void;
    /** Register cleanup, run in reverse order after draining. */
    defer(dispose: () => void | PromiseLike<void>): void;
}

/** The installation a workload runs as: its space, its build, the publishers of its copies, and the directory through its cell. */
export interface InstallationContext {
    /** The installation's identifier. */
    readonly id: string;
    /** The installation's space. */
    readonly scope: string;
    /** The build the installation's deployment runs, which reads the package's own files such as its catalogs. */
    readonly build: BuildReader;
    /** The publisher of the copies of the space's chain and the universe's rows it reads: its cell. */
    readonly publisher: Publisher;
    /** Connect to the installation of the package at an address `<installation>.<space>` as a publisher. */
    publisherAt(address: string): Publisher;
    /** The directory, through the space's cell. */
    readonly directory: CellDirectory;
}

/** The directory as an installation sees it through its cell: people's homes and locales, and the addresses of projected rows. */
export interface CellDirectory {
    /** Decide whether a person lives in a space. */
    isHome(subject: string, space: string): Promise<boolean>;
    /** Read the language and region a person set, absent while unset. */
    locale(subject: string): Promise<LocaleTag | undefined>;
    /** Record that the installation sends a person projected rows. */
    address(recipient: string): Promise<void>;
}

/** The audit history a workload delivers its calls to, in batches of one transaction each. */
export interface AuditHistory {
    /** Store a batch's calls once in one transaction. */
    ingest(
        batch: { readonly calls: readonly unknown[] },
        options?: { signal?: AbortSignal },
    ): Promise<unknown>;
}

/** The services and triggers of one workload. */
export interface WorkloadImplementation {
    /** The service implementations. */
    readonly services: readonly ServiceImplementation[];
    /** The triggers of the workload's package, whose signed requests the workload receives. */
    readonly triggers?: readonly Trigger[];
}
