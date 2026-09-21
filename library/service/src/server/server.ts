import type { Health } from "../health/health.ts";
import type { Context, ServiceHandler } from "./handler.ts";
import { reportError } from "./error.ts";

/** A host-managed HTTP service with readiness and streaming-aware draining. */
export class Server<State extends Context> implements AsyncDisposable {
    /** Current health, shared with optional health procedures. */
    readonly health: Health;
    /** Host settings retained until shutdown. */
    readonly #options: ServerOptions<State>;
    /** Accepted requests, including responses still being consumed. */
    readonly #requests = new Set<AbortController>();
    /** Notification when accepted requests complete. */
    readonly #drained = Promise.withResolvers<void>();
    /** The single shutdown attempt. */
    #closing?: Promise<void>;

    /** Retain the host's handler and lifecycle settings. */
    private constructor(options: ServerOptions<State>) {
        // reject delays that overflow the runtime's signed 32 bit timer
        if (
            !Number.isInteger(options.drainTimeout) ||
            options.drainTimeout <= 0 ||
            options.drainTimeout > 2 ** 31 - 1
        ) {
            throw new RangeError(
                "drain timeout must be a positive integer within the runtime timer limit",
            );
        }

        // retain the handler and initial health
        this.#options = options;
        this.health = options.handler.health;
    }

    /** Initialize resources before publishing readiness. */
    static async start<State extends Context>(
        options: ServerOptions<State>,
    ): Promise<Server<State>> {
        // publish readiness after initialization completes
        const server = new Server(options);
        server.health.set("starting");
        try {
            await options.initialize?.();
            server.health.set("serving");
        } catch (error) {
            // release partially initialized resources before reporting failure
            server.health.set("stopped");
            try {
                await options.dispose?.();
            } catch (cleanup) {
                throw new AggregateError(
                    [error, cleanup],
                    "service initialization and cleanup failed",
                );
            }

            throw error;
        }

        return server;
    }

    /** Dispatch probes and authorized requests, retaining streamed response lifetimes. */
    async fetch(request: Request): Promise<Response> {
        // answer probes and reject application requests during shutdown
        const probe = this.health.probe(request);
        if (probe) {
            return probe;
        }
        if (this.#closing || this.health.status !== "serving") {
            return new Response(null, { status: 503 });
        }

        // retain cancellation across authentication and response consumption
        const controller = new AbortController();
        this.#requests.add(controller);
        const signal = AbortSignal.any([request.signal, controller.signal]);
        const accepted = new Request(request, { signal });
        try {
            // authenticate before dispatching the request
            const context = await this.#options.context(accepted);
            signal.throwIfAborted();
            const result = await this.#options.handler.handle(accepted, { context });
            const response = result.matched ? result.response : new Response(null, { status: 404 });

            return this.#respond(response, controller, signal);
        } catch (error) {
            // preserve cancellation and serialize application failures
            this.#finish(controller);
            if (signal.aborted) {
                throw error;
            }

            // translate application failures to their public HTTP representation
            const failure = reportError(error);

            return Response.json(failure.toJSON(), { status: failure.status });
        }
    }

    /** Refuse new work, drain accepted requests, and dispose resources once. */
    close(): Promise<void> {
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Close the service. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Retain a response until its body completes, fails, or is cancelled. */
    #respond(response: Response, controller: AbortController, signal: AbortSignal): Response {
        // complete requests without response bodies immediately
        if (!response.body) {
            this.#finish(controller);

            return response;
        }

        // account for response bodies until they complete, fail, or are cancelled
        const reader = response.body.getReader();
        const finish = () => {
            signal.removeEventListener("abort", abort);
            this.#finish(controller);
        };

        // cancel the producer before releasing the request
        const abort = () => {
            void reader
                .cancel(signal.reason)
                .catch((error) => {
                    reportError(error);
                    responseError = error;
                })
                .finally(finish);
        };

        // forward cancellation and preserve stream errors
        let responseError: unknown;
        signal.addEventListener("abort", abort, { once: true });
        if (signal.aborted) {
            abort();
        }

        // release the request after its response stream settles
        const body = new ReadableStream<Uint8Array>({
            async pull(stream) {
                try {
                    // propagate cancellation before forwarding the next chunk
                    const next = await reader.read();
                    if (responseError !== undefined) {
                        throw responseError;
                    }
                    signal.throwIfAborted();

                    // close a completed stream or forward its next chunk
                    if (next.done) {
                        finish();
                        stream.close();
                    } else {
                        stream.enqueue(next.value);
                    }
                } catch (error) {
                    finish();
                    stream.error(error);
                }
            },
            async cancel(reason) {
                try {
                    await reader.cancel(reason);
                } finally {
                    finish();
                }
            },
        });

        return new Response(body, {
            status: response.status,
            statusText: response.statusText,
            headers: response.headers,
        });
    }

    /** Release a request and wake shutdown when all requests finish. */
    #finish(controller: AbortController): void {
        // wake shutdown after the last response releases its request
        this.#requests.delete(controller);
        if (this.#closing && !this.#requests.size) {
            this.#drained.resolve();
        }
    }

    /** Drain within the deadline and leave forced termination to the host. */
    async #close(): Promise<void> {
        // refuse new requests and wait for accepted work to release its resources
        this.health.set("draining");
        if (!this.#requests.size) {
            this.#drained.resolve();
        }

        // release resources after every accepted request finishes
        const completion = this.#drained.promise.then(async () => {
            try {
                await this.#options.dispose?.();
            } catch (error) {
                // report cleanup failures even when the shutdown deadline already expired
                reportError(error);
                throw error;
            } finally {
                this.health.set("stopped");
            }
        });

        // abort overdue requests while retaining cleanup after they settle
        const deadline = Promise.withResolvers<never>();
        const timer = setTimeout(() => {
            const error = new DOMException("Service drain deadline exceeded.", "TimeoutError");
            for (const controller of this.#requests) {
                controller.abort(error);
            }

            deadline.reject(error);
        }, this.#options.drainTimeout);

        // bound the caller's wait while cleanup retains its own completion promise
        try {
            await Promise.race([completion, deadline.promise]);
        } finally {
            clearTimeout(timer);
        }
    }
}

/** Host-provided initialization, authorization, and shutdown behavior. */
export interface ServerOptions<State extends Context> {
    /** Implemented and instrumented service handler. */
    handler: ServiceHandler<State>;
    /** Authenticate each request and construct its authorized context. */
    context(request: Request): State | Promise<State>;
    /** Complete initialization before accepting application requests. */
    initialize?(): Promise<void>;
    /** Release resources after accepted requests finish. */
    dispose?(): Promise<void>;
    /** Maximum graceful drain time before aborting outstanding requests. */
    drainTimeout: number;
}
