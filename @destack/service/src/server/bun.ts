import { serve, type Server } from "bun";

/** A request handler served on one network address. */
export interface Endpoint {
    /** The network interface. */
    readonly hostname: string;
    /** The port, or zero for an ephemeral one. */
    readonly port: number;
    /** Handle one request. */
    fetch(request: Request): Response | Promise<Response>;
}

/** The endpoints and lifecycle of one process. */
export interface ProcessOptions {
    /** The endpoints to listen on. */
    readonly endpoints: readonly Endpoint[];
    /** The signal aborted on shutdown. */
    readonly signal: AbortSignal;
    /** Request shutdown. */
    shutdown(): void;
    /** Drain requests and release the runtime. */
    close(): Promise<void>;
    /** Publish the listener addresses, in endpoint order. */
    ready?(addresses: readonly URL[]): Promise<void>;
}

/** Serve each endpoint until shutdown, then drain and close. */
export async function serveProcess(options: ProcessOptions): Promise<void> {
    // shut down on process signals
    const listeners: Server<undefined>[] = [];
    const stopped = Promise.withResolvers<void>();
    const stop = () => stopped.resolve();
    const shutdown = () => options.shutdown();
    options.signal.addEventListener("abort", stop, { once: true });
    process.on("SIGINT", shutdown);
    process.on("SIGTERM", shutdown);
    try {
        // open every listener
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
        // drain requests and close
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
