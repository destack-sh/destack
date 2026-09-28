import type { Service } from "../declare/service.ts";
import type { PackageId } from "@destack/package";
import type { Health } from "../health/health.ts";
import { ServiceHandler, type HandlerOptions, type Router } from "./handler.ts";
import { isProcedure } from "@orpc/server";
import { ProcedureAccess } from "../procedure/procedure.ts";
import { Capability, delegationChain } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { Caller } from "../authentication/index.ts";
import type { ServiceRouter } from "../service/index.ts";
import { ServiceError } from "../error/index.ts";
import { BOOKMARK_HEADER, type Bookmark } from "../bookmark/index.ts";
import { CAPABILITY_HEADER, ServiceContext, type ServiceAccess } from "./context.ts";
import type { ProcedureCall } from "./access.ts";
import { reportError } from "./error.ts";
import { MAX_TIMER_DELAY } from "../timer/index.ts";

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
        for (const callback of ["authenticate", "authorizeHost"] as const) {
            if (typeof options[callback] !== "function") {
                throw new TypeError(`${callback} must be configured before starting a server`);
            }
        }

        // reject delays that overflow the runtime's signed 32 bit timer
        if (
            !Number.isInteger(options.drainTimeout) ||
            options.drainTimeout <= 0 ||
            options.drainTimeout > MAX_TIMER_DELAY
        ) {
            throw new RangeError(
                "drain timeout must be a positive integer within the runtime timer limit",
            );
        }

        // require a policy declaring every permission a procedure requires
        requirePolicies(options.router, options.access);

        // construct the handler deciding every call through the service's policies and the host
        this.#options = options;
        this.health = options.health;
        this.#handler = new ServiceHandler(options.router, {
            ...options,
            authorize: (call) => Server.#authorize(call, options),
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

        // refuse application requests while draining
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
                response = attachBookmark(response, context.observed);
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
        // keep responses unchanged without a response policy
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
            const authenticated = await options.authenticate(request);
            authenticated?.requireCurrent(
                options.audience,
                Date.now(),
                options.scope ?? authenticated.authentication.scope,
            );
            caller = authenticated;
        } catch (error) {
            authenticationError = error ?? new ServiceError("UNAUTHORIZED");
        }

        // digest the capabilities the request presents, keeping no secrets
        const presented = (request.headers.get(CAPABILITY_HEADER) ?? "")
            .split(",")
            .map((secret) => secret.trim())
            .filter((secret) => secret.length > 0);
        const capabilities = await Promise.all(
            presented.map((secret) => Capability.digest(secret)),
        ).catch((error: unknown) => {
            throw new ServiceError("UNAUTHORIZED", { message: "invalid capability", cause: error });
        });

        return new ServiceContext(request, {
            audience: options.audience,
            scope: options.scope ?? caller?.authentication.scope,
            caller,
            resources: options.resources,
            access: options.access,
            authenticationError,
            capabilities,
        });
    }

    /** Enforce identity, credential restrictions and current installation policy. */
    static async #authorize(
        call: ProcedureCall<ServiceContext>,
        options: ServerOptions,
    ): Promise<void> {
        // reject invalid credentials on public routes and require identity on protected routes
        if (call.access.authentication !== "public") {
            call.context.requireCaller();
        }

        // read current access for the call, and again before each value of a stream
        call.context.authorization?.renew();

        // decide a required permission on the call's target through the service's policies, recording the target for the handler
        const permission = call.access.permission;
        if (permission !== null) {
            const target = await options.access!.target!(call);
            await call.context.authorization!.require(permission, target);
            call.context.target = target;
        }
        // reject invalid credentials and validate the caller's delegation chain in the service's scope
        else {
            delegationChain(call.context.access());
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
            const error = new DOMException("service drain deadline exceeded", "TimeoutError");
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
    /** The one scope the service serves, such as a space, host or account; absent for regional services serving many. */
    scope?: string;
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
    /** The policies deciding the procedures that require a permission and the permissions handlers require. */
    access?: ServiceAccess;
    /** Dispatch an additional HTTP protocol with its own authentication, or return undefined. */
    route?(request: Request): Promise<Response | undefined>;
}

/** Require a policy of the service's authorizer to declare every permission a procedure requires. */
function requirePolicies(router: unknown, access: ServiceAccess | undefined): void {
    // check each procedure's permission
    if (isProcedure(router)) {
        const permission = ProcedureAccess.parse(router["~orpc"].meta).permission;
        if (permission === null) {
            return;
        }

        // require a target and a policy declaring the permission
        if (access?.target === undefined) {
            throw new TypeError(
                `procedures requiring ${permission.name} need service access with targets`,
            );
        }
        if (
            !Object.hasOwn(
                access.authorizer.policy(permission).definition.permissions,
                permission.name,
            )
        ) {
            throw new TypeError(`no policy declares ${permission.type}.${permission.name}`);
        }
    }
    // check each nested router
    else if (router !== null && typeof router === "object") {
        for (const child of Object.values(router)) {
            requirePolicies(child, access);
        }
    }
}

/** Attach the watermarks a request's writes reached to its response. */
function attachBookmark(response: Response, observed: Bookmark): Response {
    // keep a response without writes unchanged
    if (observed.watermarks.length === 0) {
        return response;
    }

    // copy the response with the bookmark header
    const headers = new Headers(response.headers);
    headers.set(BOOKMARK_HEADER, observed.format());

    return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers,
    });
}
