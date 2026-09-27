import { serve, type Server } from "bun";

/** A request handler served on one network address. */
export interface Endpoint {
    /** The network interface selected by the host. */
    readonly hostname: string;
    /** The listening port, or zero to allocate an ephemeral port. */
    readonly port: number;
    /** Handle one request. */
    fetch(request: Request): Response | Promise<Response>;
}

/** Endpoints served by one process and the lifecycle they share. */
export interface ProcessOptions {
    /** The endpoints to listen on. */
    readonly endpoints: readonly Endpoint[];
    /** Aborted when the served runtime begins shutdown. */
    readonly signal: AbortSignal;
    /** Request shutdown, as process signals do. */
    shutdown(): void;
    /** Drain accepted requests and release the served runtime. */
    close(): Promise<void>;
    /** Publish runtime discovery once every listener is available, in endpoint order. */
    ready?(addresses: readonly URL[]): Promise<void>;
}

/** Listen on each endpoint until cooperative or process shutdown, then drain and close. */
export async function serveProcess(options: ProcessOptions): Promise<void> {
    // translate process signals into cooperative shutdown
    const listeners: Server<undefined>[] = [];
    const stopped = Promise.withResolvers<void>();
    const stop = () => stopped.resolve();
    const shutdown = () => options.shutdown();
    options.signal.addEventListener("abort", stop, { once: true });
    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
    try {
        // open every listener before publishing discovery
        for (const endpoint of options.endpoints) {
            listeners.push(
                serve({
                    hostname: endpoint.hostname,
                    port: endpoint.port,
                    fetch: (request) => endpoint.fetch(request),
                }),
            );
        }
        await options.ready?.(listeners.map((listener) => listener.url));
        if (!options.signal.aborted) {
            await stopped.promise;
        }
    } finally {
        // drain accepted requests before closing listeners and releasing subscriptions
        try {
            await options.close();
        } finally {
            try {
                await Promise.all(listeners.map((listener) => listener.stop(true)));
            } finally {
                options.signal.removeEventListener("abort", stop);
                process.off("SIGINT", shutdown);
                process.off("SIGTERM", shutdown);
            }
        }
    }
}
