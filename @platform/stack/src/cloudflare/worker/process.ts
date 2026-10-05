import { Host } from "@destack/account/object";
import type { Database, DatabaseConnection } from "@destack/db";
import { postgresConnector } from "@destack/db/postgres";
import type { PackageId } from "@destack/package";
import type { ResourceContext } from "@destack/resource/context";
import { type Service, ServiceMount } from "@destack/service";
import type { Authentication } from "@destack/service/authentication";
import type { Alarm } from "@destack/service/control";
import { telemetry } from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import { CallKey } from "@destack/service/request";
import type { DurableInstance } from "@destack/service/cloudflare";
import { type AuditHistory, type Workload, WorkloadInstance } from "@destack/service/workload";

/** The log records of the processes' workloads. */
const { log } = telemetry.scope(import.meta.destack.package);

/** The longest drain on shutdown, in milliseconds. */
const DRAIN_MILLISECONDS = 5000;

/** A Durable Object namespace, as a Worker reaches the objects of its process. */
export interface ObjectNamespace {
    /** Derive the identifier of the object of a name. */
    idFromName(name: string): unknown;
    /** Reach an object, hinting where a new one starts. */
    get(
        id: unknown,
        options?: { readonly locationHint?: string },
    ): { fetch(request: Request): Promise<Response> };
    /** Narrow the namespace to the objects kept in a jurisdiction. */
    jurisdiction(name: "eu"): ObjectNamespace;
}

/** The bindings and variables every placed process's Worker reads, as its wrangler configuration declares them. */
export interface ProcessEnvironment {
    /** The namespace of the process's one Durable Object, which runs its workload. */
    readonly WORKLOAD: ObjectNamespace;
    /** The process's PostgreSQL database through Hyperdrive. */
    readonly DATABASE: { readonly connectionString: string };
    /** The process's name, which names its Durable Object. */
    readonly DESTACK_PROCESS: string;
    /** The jurisdiction keeping the process's Durable Object, empty outside one. */
    readonly DESTACK_JURISDICTION: "" | "eu";
    /** The location hint starting the Durable Object near its database, such as weur. */
    readonly DESTACK_LOCATION: string;
    /** The universe's issuer, the account service's origin. */
    readonly DESTACK_ISSUER: string;
    /** The key the process's journals fingerprint sensitive inputs under, 32 bytes as hexadecimal. */
    readonly DESTACK_CALL_KEY: string;
}

/** What a process's workload starts with on workerd. */
export interface WorkerProcessOptions {
    /** The workload's resources. */
    readonly resources: ResourceContext;
    /** The audit history the workload's outboxes deliver to, absent for a workload journaling only in its own database. */
    readonly history?: AuditHistory;
    /** The key the workload's journals fingerprint sensitive inputs under. */
    readonly callKey: CallKey;
    /** The Durable Object's alarm, waking the workload's controllers. */
    readonly alarm: Alarm;
    /** Verify a caller of a service the workload serves. */
    authenticate(request: Request, audience: PackageId): Promise<Authentication | null>;
}

/** A placed process on workerd: its workload instance in the process's Durable Object, serving each service below its mount. */
export class WorkerProcess implements DurableInstance {
    /** The running workload. */
    readonly #instance: WorkloadInstance;
    /** The service each mounted package serves. */
    readonly #mounts: ReadonlyMap<string, Service>;
    /** The service answering outside every mount, absent for a process answering only its mounts. */
    readonly #root: Service | undefined;

    /** Keep a started workload with its mounts. */
    private constructor(
        instance: WorkloadInstance,
        mounts: ReadonlyMap<string, Service>,
        root: Service | undefined,
    ) {
        this.#instance = instance;
        this.#mounts = mounts;
        this.#root = root;
    }

    /** Start a workload in the process's Durable Object, answering outside its mounts through a root service when given. */
    static async start(
        workload: Workload,
        options: WorkerProcessOptions,
        root?: Service,
    ): Promise<WorkerProcess> {
        // start the workload, reporting background failures to the Worker's logs
        const instance = await WorkloadInstance.start(workload, {
            report: (error) => log.error("workload.failed", telemetry.exceptionAttributes(error)),
            resources: options.resources,
            callKey: options.callKey,
            ...(options.history === undefined ? {} : { history: options.history }),
            alarm: options.alarm,
            service: (service) => ({
                audience: service.package.id,
                authenticate: (request) => options.authenticate(request, service.package.id),
                authorizeHost: (call) => Host.authorize(call),
                drainTimeout: DRAIN_MILLISECONDS,
                instance: workload.name,
            }),
        });

        // mount each service it serves
        const mounts = new Map(instance.services.map((service) => [service.package.id, service]));

        return new WorkerProcess(instance, mounts, root);
    }

    /** Forward a request to the process's Durable Object, kept in its jurisdiction and started near its database. */
    static forward(environment: ProcessEnvironment, request: Request): Promise<Response> {
        const namespace =
            environment.DESTACK_JURISDICTION === ""
                ? environment.WORKLOAD
                : environment.WORKLOAD.jurisdiction(environment.DESTACK_JURISDICTION);
        const id = namespace.idFromName(environment.DESTACK_PROCESS);

        return namespace.get(id, { locationHint: environment.DESTACK_LOCATION }).fetch(request);
    }

    /** Open the process's database through Hyperdrive as its sole writer, refusing one its release has not migrated. */
    static database(
        environment: ProcessEnvironment,
        declaration: Database,
    ): Promise<DatabaseConnection> {
        return postgresConnector.connect(
            { reference: environment.DATABASE.connectionString },
            declaration,
        );
    }

    /** Read the process's journal key. */
    static callKey(environment: ProcessEnvironment): CallKey {
        const key = CallKey.import(Uint8Array.fromHex(environment.DESTACK_CALL_KEY));

        return () => key;
    }

    /** Hand a request to the service its mount names, or to the root service outside every mount. */
    fetch(request: Request): Promise<Response> {
        // find the mounted service, or the root service outside every mount
        const routed = ServiceMount.route(request);
        const service = routed === undefined ? this.#root : this.#mounts.get(routed.packageId);
        if (service === undefined) {
            return Promise.resolve(new Response(null, { status: 404 }));
        }

        return this.#instance.fetch(service, routed?.request ?? request);
    }

    /** Run the controllers at the object's alarm until no key is due now or reconciling, or until a deadline. */
    alarm(deadline: number): Promise<void> {
        return this.#instance.alarm(deadline);
    }
}
