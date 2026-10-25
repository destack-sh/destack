import type { Environment } from "@destack/environment";
import type { Channel, DatabaseConnection, Table } from "@destack/db";
import type { Identifier, JsonValue } from "@destack/schema";
import type { Publisher, Uplink } from "@destack/sync";
import {
    type SandboxName,
    type ComputeDefinition,
    ModuleMetadata,
    type PackageId,
} from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import { type BuildReader, WorkloadDefinition } from "@destack/package/manifest";
import type { ResourceContext } from "@destack/resource/context";
import type { PackageHandler } from "../server/index.ts";
import type { Deriver } from "@destack/identity";
import { type CallKey } from "../audit/key.ts";
import type { Hibernation, SocketHandler } from "./socket.ts";

/** A declared unit of deployment. */
export interface Workload extends Declaration {
    /** The compute settings of each instance. */
    readonly compute?: ComputeDefinition;
    /** The package's sandbox abilities the workload uses, every one when absent. */
    readonly sandbox?: readonly SandboxName[];
    /** Start one instance and return its packages and handlers. */
    readonly start: (
        context: WorkloadContext,
    ) => WorkloadImplementation | Promise<WorkloadImplementation>;
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

/** Where an instance stores its ephemeral objects. */
export interface EphemeralHost {
    /** Open the memory database with tables. */
    readonly open: (
        tables: readonly Table[],
    ) => Promise<DatabaseConnection & { readonly close: () => Promise<void> }>;
    /** Open the channel replicating ephemeral writes between the installation's instances. */
    readonly channel: (durable: DatabaseConnection) => Channel<unknown>;
}

/** The context of a starting workload. */
export interface WorkloadContext {
    /** The installation's resource and package clients. */
    readonly resources: ResourceContext;
    /** The server context of the installation the workload runs as, absent outside a space. */
    readonly server?: InstalledServerContext;
    /** Read the key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** The environment the workload runs in: its clock, randomness, crypto and network. */
    readonly environment: Environment;
    /** Where the instance stores its ephemeral objects, absent outside a runner. */
    readonly ephemeral?: EphemeralHost;
    /** The WebSockets the host hibernates for the instance, absent where it hibernates none. */
    readonly hibernation?: Hibernation;
    /** The shutdown signal. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    readonly shutdown: () => void;
    /** Start the workload again, serving the new start once it runs and draining this one. */
    readonly reload: () => Promise<void>;
    /** Report a failure of the workload's background work to its host. */
    readonly report: (error: unknown) => void;
    /** Register cleanup, run in reverse order after draining. */
    readonly defer: (dispose: () => void | PromiseLike<void>) => void;
}

/** The context a package's server runs in: a base installation's, or an installed one's. */
export type ServerContext = BaseServerContext | InstalledServerContext;

/** The context of a base installation, born with its scope and shipped with the platform. */
export interface BaseServerContext {
    /** The base installation, born with its scope. */
    readonly origin: "base";
    /** The installation's identifier. */
    readonly installation: Identifier<"installation">;
    /** The installation's partition scope: its space, machine, device or the universe. */
    readonly scope: string;
    /** The package's required capabilities, which a base installation runs with. */
    readonly capabilities: readonly string[];
}

/** The context of a declared or installed installation: its build, bindings, keys and egress. */
export interface InstalledServerContext {
    /** The installation its scope's stack declares, or one someone installed. */
    readonly origin: "declared" | "installed";
    /** The installation's identifier. */
    readonly installation: Identifier<"installation">;
    /** The installation's name in its scope, its address there as `<name>--<scope>`. */
    readonly name: string;
    /** The installation's partition scope: its space, machine, device or the universe. */
    readonly scope: string;
    /** The one partition of the installation's database the instance serves. */
    readonly partition?: string;
    /** The build the installation's deployment runs. */
    readonly build: BuildReader;
    /** The installation's resolved value of each setting its package declares. */
    readonly settings: Readonly<Record<string, JsonValue>>;
    /** The package's capabilities the installation runs with: required and allowed ones. */
    readonly capabilities: readonly string[];
    /** The installation's bound resources and secrets, each read through its declaration. */
    readonly resources: ResourceContext;
    /** Derive the installation's keys and secrets from its root, rooted in its space's identity. */
    readonly keys: Deriver;
    /** The publisher of the replicas of the space's chain and the universe's rows it reads. */
    readonly publisher: Uplink;
    /** Connect to the replicas an installation at `<installation>--<space>` publishes. */
    readonly publisherAt: (address: string, packageId: PackageId) => Publisher;
    /** Call the packages at an address through the host's egress as the installation. */
    readonly at: (address: string) => {
        readonly url: string;
        readonly fetch: (request: Request) => Promise<Response>;
    };
    /** Send a request to any other origin, such as a Git remote, through the host. */
    readonly fetch: (request: Request) => Promise<Response>;
    /** The URL callers call the installation at, its packages mounted below. */
    readonly url: string;
    /** The clock, network and randomness the installation runs in. */
    readonly environment: Environment;
    /** Run the installation's controllers once this settles, at once when absent. */
    readonly activation?: Promise<void>;
}

/** The packages of one workload and what it does with its host's deliveries. */
export interface WorkloadImplementation {
    /** The packages it serves, each with its procedures and enforcement. */
    readonly packages: readonly PackageHandler[];
    /** What the workload does with the webhook deliveries its host forwards. */
    readonly webhooks?: WebhookHandler;
    /** What the workload does with its hibernated sockets' messages and closes. */
    readonly sockets?: SocketHandler;
    /** What the workload does with its space store's requests to run its objects' hooks. */
    readonly hooks?: HookHandler;
}

/** What a workload does with the webhook deliveries its host forwards, below the webhook path. */
export interface WebhookHandler {
    /** Answer a delivery at its path below the webhook path. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** What a workload does with its space store's requests to run its objects' hooks. */
export interface HookHandler {
    /** Answer a request to run a hook. */
    readonly fetch: (request: Request) => Promise<Response>;
}
