import type { Channel, DatabaseConnection, Table } from "@destack/db";
import type { Clock, JsonValue } from "@destack/schema";
import type { Publisher, Uplink } from "@destack/sync";
import {
    type CapabilityName,
    type ComputeDefinition,
    ModuleMetadata,
    type PackageId,
} from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { type BuildReader, WorkloadDefinition } from "@destack/package/manifest";
import type { Permission, PermissionRequest, PermissionScope } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { LocaleTag } from "@destack/locale";
import type { ServiceImplementation } from "../server/index.ts";
import type { RunClient, Trigger } from "../trigger/index.ts";
import type { Deriver } from "@destack/identity";
import { type CallKey } from "../request/index.ts";
import type { Hibernation, SocketHandler } from "./socket.ts";

/** A declared unit of deployment. */
export interface Workload extends Declaration {
    /** The compute settings of each instance. */
    readonly compute?: ComputeDefinition;
    /** The package's capabilities the workload uses, every one when absent. */
    readonly capabilities?: readonly CapabilityName[];
    /** The kind permissions the workload requests by scope, consented to at install and granted exactly. */
    readonly permissions?: PermissionRequest;
    /** Start one instance and return its services and webhooks. */
    start(context: WorkloadContext): WorkloadImplementation | Promise<WorkloadImplementation>;
}

/** Declare a workload. */
export function defineWorkload(
    definition: Omit<WorkloadDefinition, "permissions"> &
        Pick<Workload, "start"> & {
            /** The kind permissions the workload requests by scope, consented to at install and granted exactly. */
            readonly permissions?: { readonly [Scope in PermissionScope]?: readonly Permission[] };
        },
    module?: ModuleMetadata,
): Workload {
    // stamp the declaring package
    const owner = ModuleMetadata.require(module, "defineWorkload").package;
    const { start, ...fields } = definition;

    return Object.freeze({ ...WorkloadDefinition.parse(fields), start, package: owner });
}

/** Where an instance keeps its ephemeral objects: a memory database and a channel between instances. */
export interface EphemeralHost {
    /** Open the memory database with tables. */
    open(tables: readonly Table[]): Promise<DatabaseConnection & { close(): Promise<void> }>;
    /** Open the channel replicating ephemeral writes between the installation's instances. */
    channel(durable: DatabaseConnection): Channel<unknown>;
}

/** The context of a starting workload. */
export interface WorkloadContext {
    /** The installation's resource and service clients. */
    readonly resources: ResourceContext;
    /** The audit history the workload's outboxes deliver to, absent for a workload journaling only in its own database. */
    readonly history?: AuditHistory;
    /** The installation the workload runs as, absent outside a space. */
    readonly installation?: InstallationContext;
    /** The machine recording the installation's runs: sent calls and the calls of its triggers' causes, absent outside a space. */
    readonly runs?: RunClient;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** The clock the workload runs at. */
    readonly clock: Clock;
    /** Derive the installation's own secrets under labels, the same on every instance of the installation. */
    readonly deriver: Deriver;
    /** Where the instance keeps its ephemeral objects, absent outside a runner. */
    readonly ephemeral?: EphemeralHost;
    /** The WebSockets the host keeps open for the instance, absent where the host keeps none. */
    readonly hibernation?: Hibernation;
    /** The shutdown signal. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Report a failure of the workload's background work to its host. */
    report(error: unknown): void;
    /** Register cleanup, run in reverse order after draining. */
    defer(dispose: () => void | PromiseLike<void>): void;
}

/** The installation a workload runs as: its space, its build and settings, the publishers of its copies, and the directory through its machine. */
export interface InstallationContext {
    /** The installation's identifier. */
    readonly id: string;
    /** The installation's space. */
    readonly scope: string;
    /** The one partition of the installation's database the instance serves, every partition when absent. */
    readonly partition?: string;
    /** The build the installation's deployment runs, which reads the package's own files such as its catalogs. */
    readonly build: BuildReader;
    /** The installation's resolved value of each setting its package declares, by `<package>/<name>`. */
    readonly settings: Readonly<Record<string, JsonValue>>;
    /** The publisher of the copies of the space's chain and the universe's rows it reads, receiving their changes: its machine. */
    readonly publisher: Uplink;
    /** Connect to the copies an installation at an address `<installation>--<space>` publishes through the object API of a package's service, which decides each follow by its shape's audience. */
    publisherAt(address: string, packageId: PackageId): Publisher;
    /** Reach the services at an address through the host's egress as the installation: a space's own at `<space>`, an installation's at `<installation>--<space>`. */
    at(address: string): { readonly url: string; fetch(request: Request): Promise<Response> };
    /** The directory, through the space's machine. */
    readonly directory: InstallationDirectory;
}

/** The directory as an installation sees it through its machine: people's homes and locales, the addresses of projected rows, and the keys messages are sealed to. */
export interface InstallationDirectory {
    /** Decide whether a person lives in a space. */
    isHome(subject: string, space: string): Promise<boolean>;
    /** Read the language and region a person reads. */
    locale(subject: string): Promise<LocaleTag>;
    /** Record that the installation sends a person projected rows. */
    address(recipient: string): Promise<void>;
    /** Read the public key the messages of a space are sealed to, its identity's message key. */
    messageKey(space: string): Promise<string>;
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
    /** What the workload does with its held sockets' messages and closes. */
    readonly sockets?: SocketHandler;
}
