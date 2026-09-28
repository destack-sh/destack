import type { ResourceContext } from "@destack/resource/context";
import { reference } from "@destack/package/declare";
import { Server, type ServerOptions, type ServiceImplementation } from "../server/index.ts";
import type { Service } from "../declare/service.ts";
import type {
    Trigger,
    TriggerEvent,
    TriggerHandler,
    TriggerImplementation,
} from "../trigger/index.ts";
import { Health } from "../health/index.ts";
import { ServiceError } from "../error/index.ts";
import type { Workload } from "./workload.ts";

/** A running workload instance. */
export class WorkloadInstance implements AsyncDisposable {
    /** The services by declaration key. */
    readonly #services = new Map<string, Server>();
    /** The trigger handlers by key. */
    readonly #triggers = new Map<string, TriggerImplementation>();
    /** The shutdown controller. */
    readonly #controller = new AbortController();
    /** The resources released after draining. */
    readonly #cleanup = new AsyncDisposableStack();
    /** The shutdown. */
    #closing?: Promise<void>;

    /** Start a workload. */
    static async start(
        workload: Workload,
        options: WorkloadInstanceOptions,
    ): Promise<WorkloadInstance> {
        // clean up a failed start
        const instance = new WorkloadInstance();
        try {
            // start the workload
            const implementation = await workload.start({
                resources: options.resources,
                signal: instance.#controller.signal,
                shutdown: () => instance.shutdown(),
                defer: (dispose) => instance.#cleanup.defer(dispose),
            });

            // start each service
            for (const service of implementation.services) {
                instance.signal.throwIfAborted();
                const key = keyOf(service.service);
                if (instance.#services.has(key)) {
                    throw new TypeError(`duplicate workload service: ${key}`);
                }

                // start the server
                const server = Server.start({
                    ...service,
                    ...options.service(service.service),
                    health: new Health(service.service.name),
                    resources: options.resources,
                });
                instance.#services.set(key, server);
            }

            // register the trigger handlers
            for (const handler of implementation.triggers ?? []) {
                const key = triggerKey(handler.trigger);
                if (instance.#triggers.has(key)) {
                    throw new TypeError(`duplicate workload ${key}`);
                }
                instance.#triggers.set(key, handler);
            }

            // reject a cancelled start
            instance.signal.throwIfAborted();
        } catch (error) {
            // release a failed start
            try {
                await instance.close();
            } catch (cleanup) {
                throw new AggregateError([error, cleanup], "workload startup and cleanup failed");
            }
            throw error;
        }

        return instance;
    }

    /** The shutdown signal. */
    get signal(): AbortSignal {
        return this.#controller.signal;
    }

    /** The implemented triggers, in workload order. */
    get triggers(): readonly Trigger[] {
        return [...this.#triggers.values()].map((handler) => handler.trigger);
    }

    /** Report whether the workload implements a service. */
    has(service: Service): boolean {
        return this.#services.has(keyOf(service));
    }

    /** Request shutdown. */
    shutdown(): void {
        this.#controller.abort();
    }

    /** Dispatch a request to a service. */
    fetch(service: Service, request: Request): Promise<Response> {
        // require the service
        const server = this.#services.get(keyOf(service));
        if (!server) {
            throw new ServiceError("NOT_FOUND", {
                message: `unknown workload service: ${keyOf(service)}`,
            });
        }

        return server.fetch(request);
    }

    /** Deliver one trigger event to its handler. */
    deliver<Declared extends Trigger>(
        trigger: Declared,
        event: TriggerEvent<Declared>,
        signal: AbortSignal,
    ): Promise<void> {
        // require the handler
        const key = triggerKey(trigger);
        const handler = this.#triggers.get(key) as TriggerHandler<Declared> | undefined;
        if (!handler) {
            throw new ServiceError("NOT_FOUND", { message: `unknown workload ${key}` });
        }

        return handler.handle(event, AbortSignal.any([signal, this.signal]));
    }

    /** Close the workload. */
    close(): Promise<void> {
        this.shutdown();
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Close the workload. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Close every service and resource, reporting all failures. */
    async #close(): Promise<void> {
        // drain each service
        const servers = [...this.#services.values()];
        const results = await Promise.allSettled(servers.map((server) => server.close()));
        const errors = results.flatMap((result) =>
            result.status === "rejected" ? [result.reason] : [],
        );

        // wait for the servers to stop
        await Promise.all(servers.map((server) => server.stopped));

        // release the resources
        try {
            await this.#cleanup.disposeAsync();
        } catch (error) {
            errors.push(error);
        }

        // report the failures
        if (errors.length) {
            throw new AggregateError(errors, "workload shutdown failed");
        }
    }
}

/** The hosting configuration of a workload. */
export interface WorkloadInstanceOptions {
    /** The installation's resources. */
    readonly resources: ResourceContext;
    /** Select the options of one service. */
    service(
        service: Service,
    ): Omit<ServerOptions, keyof ServiceImplementation | "health" | "resources">;
}

/** Key a declaration by its package and name. */
function keyOf(declaration: Service | Trigger): string {
    const { packageId, name } = reference(declaration);

    return `${packageId}/${name}`;
}

/** Key a trigger by its kind, package and name. */
function triggerKey(trigger: Trigger): string {
    return `${trigger.kind}: ${keyOf(trigger)}`;
}
