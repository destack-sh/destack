import type { Service } from "../declare/service.ts";
import type { PackageId } from "@destack/package";
import type { Health } from "../health/health.ts";
import { ServiceHandler, type HandlerOptions, type Router } from "./handler.ts";
import { permitsCredential, permitsDelegation } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { Caller } from "../authentication/index.ts";
import type { ServiceRouter } from "../service/index.ts";
import { ServiceError } from "../error/index.ts";
import { ServiceContext } from "./context.ts";
import type { ProcedureCall } from "./access.ts";
import { reportError } from "./error.ts";

/** A host-managed HTTP service with readiness and streaming-aware draining. */
export class Server implements AsyncDisposable {
    /** Current health, shared with optional health procedures. */
    readonly health: Health;
    /** Host settings retained until shutdown. */
    readonly #options: ServerOptions;
    /** HTTP adapter constructed from the declared router. */
    readonly #handler: ServiceHandler<ServiceContext>;
    /** Accepted requests, including responses still being consumed. */
    readonly #requests = new Set<AbortController>();
    /** Notification when accepted requests complete. */
    readonly #drained = Promise.withResolvers<void>();
    /** Completion of request draining and resource disposal, including after a timeout. */
    readonly #stopped = Promise.withResolvers<void>();
    /** The single shutdown attempt. */
    #closing?: Promise<void>;

    /** Wait until accepted requests have actually finished. */
    get stopped(): Promise<void> {
        return this.#stopped.promise;
    }

    /** Construct the HTTP handler and retain lifecycle settings. */
    private constructor(options: ServerOptions) {
        // reject missing enforcement before wrapping callbacks for the HTTP adapter
        for (const callback of ["authenticate", "authorizeHost", "authorize"] as const) {
            if (typeof options[callback] !== "function") {
                throw new TypeError(`${callback} must be configured before starting a server`);
            }
        }

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

        // construct the handler with mandatory host and application authorization
        this.#options = options;
        this.health = options.health;
        this.#handler = new ServiceHandler(options.router, {
            ...options,
            authorize: async (call) => {
                await Server.#authorize(call, options);
                await options.authorize(call);
            },
        });
    }

    /** Configure request handling and publish readiness. */
    static start(options: ServerOptions): Server {
        const server = new Server(options);
        server.health.set("serving");

        return server;
    }

    /** Dispatch probes and authorized requests, retaining streamed response lifetimes. */
    async fetch(request: Request): Promise<Response> {
        // answer probes and reject application requests during shutdown
        const probe = this.health.probe(request);
        if (probe) {
            return this.#headers(probe);
        }
        if (this.#closing || this.health.status !== "serving") {
            return this.#headers(new Response(null, { status: 503 }));
        }

        // retain cancellation across authentication and response consumption
        const controller = new AbortController();
        this.#requests.add(controller);
        const signal = AbortSignal.any([request.signal, controller.signal]);
        try {
            // dispatch additional protocols or authenticate the declared service procedures
            const accepted = new Request(request, { signal });
            let response = await this.#options.route?.(accepted);
            if (response === undefined) {
                const context = await Server.#authenticate(accepted, this.#options);
                signal.throwIfAborted();
                const result = await this.#handler.handle(accepted, { context });
                response = result.matched ? result.response : new Response(null, { status: 404 });
            }

            return this.#respond(this.#headers(response), controller, signal);
        } catch (error) {
            // preserve cancellation and serialize application failures
            this.#finish(controller);
            if (signal.aborted) {
                throw error;
            }

            // translate application failures to their public HTTP representation
            const failure = reportError(error);

            return this.#headers(Response.json(failure.toJSON(), { status: failure.status }));
        }
    }

    /** Refuse new work, drain accepted requests, and dispose resources once. */
    close(): Promise<void> {
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Apply deployment response policy to success, failure and health responses. */
    #headers(response: Response): Response {
        if (!this.#options.responseHeaders) {
            return response;
        }

        // preserve responses whose headers are immutable, including protocol redirects
        const headers = new Headers(response.headers);
        for (const [name, value] of new Headers(this.#options.responseHeaders)) {
            headers.set(name, value);
        }

        return new Response(response.body, {
            status: response.status,
            statusText: response.statusText,
            headers,
        });
    }

    /** Close the service. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Retain authentication failures for the procedure's audit path. */
    static async #authenticate(request: Request, options: ServerOptions): Promise<ServiceContext> {
        // authenticate only against the installation's trusted configuration
        let caller: Caller | null = null;
        let authenticationError: unknown;
        try {
            caller = await options.authenticate(request);
            caller?.context(
                options.audience,
                Date.now(),
                options.target ? caller.authentication.scope : options.scope,
            );
        } catch (error) {
            authenticationError = error ?? new ServiceError("UNAUTHORIZED");
        }

        return new ServiceContext(
            request,
            options.audience,
            options.target ? (caller?.authentication.scope ?? options.scope) : options.scope,
            caller,
            options.resources,
            authenticationError,
        );
    }

    /** Enforce identity, credential restrictions and current installation policy. */
    static async #authorize(
        call: ProcedureCall<ServiceContext>,
        options: ServerOptions,
    ): Promise<void> {
        // reject invalid credentials on public routes and require identity on protected routes
        let access = call.context.access();
        if (call.access.authentication !== "public") {
            call.context.requireCaller();
        }

        // constrain the operation before application authorization selects its exact objects
        const permission = call.access.permission;
        const target = options.target ? await options.target(call) : { scope: options.scope };
        if (call.context.caller) {
            access = call.context.caller.context(options.audience, Date.now(), target.scope);
        }
        if (
            permission &&
            (!permitsCredential(permission, target, access) ||
                !permitsDelegation(permission, target, access))
        ) {
            throw new ServiceError("FORBIDDEN");
        }

        // let the host authorize the call
        await options.authorizeHost(call);
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

        // report stopped after every accepted request finishes
        const completion = this.#drained.promise.then(() => {
            this.health.set("stopped");
            this.#stopped.resolve();
        });

        // abort overdue requests after the drain deadline
        const deadline = Promise.withResolvers<never>();
        const timer = setTimeout(() => {
            const error = new DOMException("Service drain deadline exceeded.", "TimeoutError");
            for (const controller of this.#requests) {
                controller.abort(error);
            }

            deadline.reject(error);
        }, this.#options.drainTimeout);

        // bound the caller's wait while draining retains its own completion promise
        try {
            await Promise.race([completion, deadline.promise]);
        } finally {
            clearTimeout(timer);
        }
    }
}

/** Authentication, authorization, resources and lifecycle for one hosted service. */
export interface ServerOptions extends Omit<ServiceImplementation, "service"> {
    /** Readiness shared with the host. */
    health: Health;
    /** Fixed receiving package identifier. */
    audience: PackageId;
    /** Default authorization scope, such as a space, host, account or global authority. */
    scope: string;
    /** Installation resource clients selected by the host. */
    resources: ResourceContext;
    /** Verify credentials; return null only when the request has no credential. */
    authenticate(request: Request): Promise<Caller | null>;
    /** Enforce installation restrictions and host-only access requirements. */
    authorizeHost(call: ProcedureCall<ServiceContext>): Promise<void>;
    /** Maximum graceful drain time in milliseconds before aborting outstanding requests. */
    drainTimeout: number;
}

/** Implemented procedures, domain enforcement and resource lifecycle. */
export interface ServiceImplementation extends Omit<HandlerOptions<ServiceContext>, "health"> {
    /** The declared service these procedures implement. */
    readonly service: Service;
    /** Application procedures receiving the standard service context. */
    router: Router<ServiceRouter, ServiceContext>;
    /** Response headers enforced on every response, including failures and probes. */
    responseHeaders?: ConstructorParameters<typeof Headers>[0];
    /** Select an exact authorization target when a service administers other spaces or objects. */
    target?(call: ProcedureCall<ServiceContext>): Promise<{ scope: string; id?: string }>;
    /** Dispatch an additional HTTP protocol with its own authentication, or return undefined. */
    route?(request: Request): Promise<Response | undefined>;
    /** Enforce application permissions, including exact objects and sharing grants. */
    authorize(call: ProcedureCall<ServiceContext>): Promise<void>;
}
