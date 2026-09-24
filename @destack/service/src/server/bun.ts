import { serve, type Server } from "bun";
import type { WorkloadInstance } from "./instance.ts";
import type { Service } from "../declare/service.ts";

/** Listen for service calls and retain the workload until cooperative or process shutdown. */
export async function runWorkload(
    workload: WorkloadInstance,
    options: ProcessOptions,
): Promise<void> {
    // translate process signals into cooperative workload cancellation
    const listeners: Server<undefined>[] = [];
    const shutdown = () => workload.shutdown();
    const stopped = Promise.withResolvers<void>();
    const stop = () => stopped.resolve();
    workload.signal.addEventListener("abort", stop, { once: true });
    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
    try {
        // expose only the services assigned addresses by this host
        const addresses = new Map<Service, URL>();
        for (const { service, hostname, port } of options.services) {
            if (!workload.has(service)) {
                throw new TypeError(
                    `unknown workload service: ${service.package.name}/${service.name}`,
                );
            }
            const listener = serve({
                hostname,
                port,
                fetch: (request) => workload.fetch(service, request),
            });
            listeners.push(listener);
            addresses.set(service, listener.url);
        }

        // publish discovery only after every listener is available
        await options.ready?.(addresses);
        if (!workload.signal.aborted) {
            await stopped.promise;
        }
    } finally {
        // stop observations before draining accepted requests and shared resources
        try {
            await workload.close();
        } finally {
            try {
                await Promise.all(listeners.map((listener) => listener.stop(true)));
            } finally {
                // release process subscriptions even if a listener fails to close
                workload.signal.removeEventListener("abort", stop);
                process.off("SIGINT", shutdown);
                process.off("SIGTERM", shutdown);
            }
        }
    }
}

/** Listener addresses and discovery publication for a Bun-hosted workload. */
export interface ProcessOptions {
    /** Declared services mapped to explicit network bindings. */
    readonly services: readonly {
        /** The declared service to expose. */
        readonly service: Service;
        /** Network interface selected by the host. */
        readonly hostname: string;
        /** Listening port, or zero to allocate an ephemeral port. */
        readonly port: number;
    }[];
    /** Publish runtime discovery after all requested listeners are available. */
    ready?(addresses: ReadonlyMap<Service, URL>): Promise<void>;
}
