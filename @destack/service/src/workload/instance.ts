import type { ResourceContext } from "@destack/resource/context";
import { reference } from "@destack/package/declare";
import { Server, type ServerOptions, type ServiceImplementation } from "../server/index.ts";
import type { Service } from "../declare/service.ts";
import type { Schedule, ScheduleImplementation } from "../schedule/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import type { Workload } from "./workload.ts";

/** A running workload instance: its services and schedules sharing one lifetime. */
export class WorkloadInstance implements AsyncDisposable {
    /** Running services keyed by their declaration reference. */
    readonly #services = new Map<string, Server>();
    /** Schedule handlers keyed by their declaration reference. */
    readonly #schedules = new Map<string, ScheduleImplementation>();
    /** Cancellation requested by the host or workload. */
    readonly #controller = new AbortController();
    /** Resources released after service requests drain. */
    readonly #cleanup = new AsyncDisposableStack();
    /** One shutdown attempt shared by all callers. */
    #closing?: Promise<void>;

    /** Initialize a workload before exposing any of its services. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
    ): Promise<WorkloadInstance> {
        // retain partial initialization until all registered cleanup completes
        const instance = new WorkloadInstance();
        try {
            // allow startup to register cleanup before acquiring the next resource
            const implementation = await workload.start({
                resources: options.resources,
                signal: instance.#controller.signal,
                shutdown: () => instance.shutdown(),
                defer: (dispose) => instance.#cleanup.defer(dispose),
            });

            // bind every service to host-selected authentication and resource clients
            for (const service of implementation.services) {
                instance.signal.throwIfAborted();
                const key = keyOf(service.service);
                if (instance.#services.has(key)) {
                    throw new TypeError(`duplicate workload service: ${key}`);
                }
                const server = Server.start({
                    ...service,
                    ...options.service(service.service),
                    health: new Health(service.service.name),
                    resources: options.resources,
                });
                instance.#services.set(key, server);
            }

            // retain schedule handlers for the host scheduler
            for (const schedule of implementation.schedules ?? []) {
                const key = keyOf(schedule.schedule);
                if (instance.#schedules.has(key)) {
                    throw new TypeError(`duplicate workload schedule: ${key}`);
                }
                instance.#schedules.set(key, schedule);
            }

            // reject cancellation requested by the final initializer before publishing services
            instance.signal.throwIfAborted();
        } catch (error) {
            // release successfully initialized services and partial startup resources
            try {
                await instance.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "workload startup and cleanup failed");
            }
            throw error;
        }

        return instance;
    }

    /** Observe cooperative shutdown without assuming the host guarantees finalization. */
    get signal(): AbortSignal {
        return this.#controller.signal;
    }

    /** Report whether the workload implements a declared service. */
    has(service: Service): boolean {
        return this.#services.has(keyOf(service));
    }

    /** Request shutdown and cancel workload background observations. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Route an invocation to a declared service without opening a socket. */
    fetch(service: Service, request: Request): Promise<Response> {
        // require an initialized service selected by the host
        const server = this.#services.get(keyOf(service));
        if (!server) {
            throw new ServiceError("NOT_FOUND", {
                message: `unknown workload service: ${keyOf(service)}`,
            });
        }

        return server.fetch(request);
    }

    /** Run one occurrence of a declared schedule. */
    run(schedule: Schedule, signal: AbortSignal): Promise<void> {
        // require a handler implemented by this workload
        const implementation = this.#schedules.get(keyOf(schedule));
        if (!implementation) {
            throw new ServiceError("NOT_FOUND", {
                message: `unknown workload schedule: ${keyOf(schedule)}`,
            });
        }

        return implementation.run(AbortSignal.any([signal, this.signal]));
    }

    /** Cancel background work and await request completion before releasing shared resources. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Release the workload when its hosting scope exits. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Preserve failures while attempting every service and resource cleanup. */
    async #close(): Promise<void> {
        // drain independently so a failed service cannot prevent another from closing
        const servers = [...this.#services.values()];
        const results = await Promise.allSettled(servers.map((server) => server.close()));
        const errors = results.flatMap((result) =>
            result.status === "rejected" ? [result.reason] : [],
        );

        // retain shared resources while a timed-out server still finishes accepted work
        await Promise.all(servers.map((server) => server.stopped));

        // release startup resources even after a service cleanup fails
        try {
            await this.#cleanup.disposeAsync();
        } catch (error) {
            errors.push(error);
        }

        // report every failure after all cleanup has been attempted
        if (errors.length) {
            throw new AggregateError(errors, "workload shutdown failed");
        }
    }
}

/** Trusted hosting configuration, separate from package service implementations. */
export interface WorkloadInstanceOptions {
    /** Host-owned installation resources, retained until the workload closes. */
    readonly resources: ResourceContext;
    /** Select authentication and execution limits for one declared service. */
    service(
        service: Service,
    ): Omit<ServerOptions, keyof ServiceImplementation | "health" | "resources">;
}

/** Key a declaration by its package and name. */
function keyOf(declaration: Service | Schedule): string {
    const { packageId, name } = reference(declaration);

    return `${packageId}/${name}`;
}
