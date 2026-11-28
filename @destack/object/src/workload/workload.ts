import {
    defineKind,
    type SandboxName,
    type ComputeDefinition,
    ModuleMetadata,
    type PackageId,
} from "@destack/package";
import type { ObjectStore } from "../server/store.ts";
import type { ControllerSet, BlobStore, DatabaseConnection } from "@destack/db/database";
import type { ExchangeSubject } from "@destack/access/token";
import type { ObjectConnection } from "../replica/connection.ts";
import type { Environment } from "@destack/environment";
import type { DatabaseSchema, Table } from "@destack/db";
import type { Identifier, JsonValue } from "@destack/schema";
import type { ObjectRemote } from "../replica/remote.ts";
import type { Declaration } from "@destack/package/declare";
import {
    type BuildLayer,
    type BuildReader,
    WorkloadDefinition,
    WorkloadDescription,
} from "@destack/package/manifest";
import type { ResourceContext } from "@destack/resource/context";
import type { PackageHandler } from "../server/index.ts";
import type { Deriver } from "@destack/identity";
import type { CallKey } from "../audit/key.ts";
import type { Hibernation, SocketHandler } from "./socket.ts";

/** The export of a server output's workload entry: the workload its runtime hosts. */
export const WORKLOAD_EXPORT = "Workload";

/** The workload declaration kind, which object defines. */
export const WORKLOAD_KIND = defineKind({ name: "workload", description: WorkloadDescription });

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

/** Recognise workloads, as a workload's module exports its own. */
export const Workload = {
    /** Report whether a module's export is a declared workload. */
    is(value: unknown): value is Workload {
        return (
            typeof value === "object" &&
            value !== null &&
            "start" in value &&
            typeof value.start === "function"
        );
    },
};

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
    /** Open the memory database of a schema with replicated tables, or of tables alone. */
    readonly open: (
        schema: DatabaseSchema | readonly Table[],
    ) => Promise<DatabaseConnection & { readonly close: () => Promise<void> }>;
}

/** The context of a starting workload. */
export interface WorkloadContext {
    /** The installation's resource and package clients. */
    readonly resources: ResourceContext;
    /** The server context of the installation the workload runs as, absent outside a space. */
    readonly server?: ServerContext;
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
    /** Serve another set of packages in this start, the replaced servers draining beside it. */
    readonly serve: (packages: readonly PackageHandler[]) => void;
    /** Register cleanup, run in reverse order after draining. */
    readonly defer: (dispose: () => void | PromiseLike<void>) => void;
}

/** An installation's server context, with its partition's database and blobs. */
export interface PackageContext extends ServerContext {
    /** The installation's database, with the tables the package declares beside its objects. */
    readonly database: DatabaseConnection;
    /** The partition's blobs, which the package's blob columns keep alive. */
    readonly blobs: BlobStore;
}

/** Give an installation's server context the database its store serves. */
export const PackageContext = {
    /** Add the store's database and blobs to an installation's context, none for own packages. */
    of(context: ServerContext | undefined, blobs: BlobStore): PackageContext | undefined {
        return context === undefined ? undefined : { ...context, database: blobs.database, blobs };
    },
};

/** The context an installation's server runs in: its build, bindings, keys and egress. */
export interface ServerContext {
    /** The installation's identifier. */
    readonly installation: Identifier<"installation">;
    /** The installation's name in its scope, its address there as `<name>--<scope>`. */
    readonly name: string;
    /** The installation's space. */
    readonly scope: string;
    /** The one partition of the installation's database the instance serves. */
    readonly partition?: string;
    /** The build the installation's deployment runs. */
    readonly build: BuildReader;
    /** The builds of the host's distribution, which the installation's stores read in place. */
    readonly distribution: BuildLayer;
    /** The installation's resolved value of each setting its package declares. */
    readonly settings: Readonly<Record<string, JsonValue>>;
    /** The package's capabilities the installation runs with: required and allowed ones. */
    readonly capabilities: readonly string[];
    /** The installation's bound resources and secrets, each read through its declaration. */
    readonly resources: ResourceContext;
    /** Derive the installation's keys and secrets from its root, rooted in its space's identity. */
    readonly keys: Deriver;
    /** The remote toward the homes of the space's chain and the universe's rows it reads. */
    readonly remote: ObjectRemote;
    /** Connect to the store of a package at `<installation>--<space>` through the host's egress. */
    readonly connect: (address: string, packageId: PackageId) => ObjectRemote;
    /** Connect to the space's stack as a subject the installation serves, through the egress. */
    readonly connectAs: (subject: ExchangeSubject) => ObjectConnection;
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
    /** Activate the installation before its controllers run, stopping once the signal aborts. */
    readonly activation?: (signal: AbortSignal) => Promise<void>;
}

/** The packages of one workload and what it does with its host's deliveries. */
export interface WorkloadImplementation {
    /** The packages it serves, each with its procedures and enforcement. */
    readonly packages: readonly PackageHandler[];
    /** The store whose jobs the start runs, absent for a workload without one. */
    readonly store?: Pick<ObjectStore, "run" | "idle">;
    /** Read the process's own jobs, run in the store's loop again at each restart. */
    readonly control?: () => readonly ControllerSet[];
    /** What the workload does with its hibernated sockets' messages and closes. */
    readonly sockets?: SocketHandler;
    /** What the workload does with its space store's requests to run its objects' hooks. */
    readonly hook?: { readonly fetch: (request: Request) => Promise<Response> };
    /** What the workload does with its host's requests to render its views as their visitors. */
    readonly view?: { readonly fetch: (request: Request) => Promise<Response> };
}
