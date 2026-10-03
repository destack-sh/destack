import type { PackageId } from "@destack/package";
import type { Health } from "../health/health.ts";
import { ServiceHandler, type HandlerOptions, type Router } from "./handler.ts";
import { isProcedure } from "@orpc/server";
import { ProcedureMeta } from "../procedure/procedure.ts";
import { LinkSecret, Caller } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { Authentication } from "../authentication/index.ts";
import type { ServiceRouter } from "../service/index.ts";
import { ServiceError } from "../error/index.ts";
import { BOOKMARK_HEADER, type Bookmark } from "../bookmark/index.ts";
import { LINK_SECRET_HEADER, ServiceContext, type ServiceAccess } from "./context.ts";
import type { ProcedureCall } from "./access.ts";
import { reportError, reportReconciliation } from "./error.ts";
import { type Alarm, ControlLoop, type Controller } from "../control/index.ts";
import { MAX_TIMER_DELAY } from "../timer/index.ts";
import { copyRequest } from "../request/index.ts";

/** A hosted HTTP service. */
export class Server implements AsyncDisposable {
    /** The health. */
    readonly health: Health;
    /** The server options. */
    readonly #options: ServerOptions;
    /** The HTTP handler. */
    readonly #handler: ServiceHandler<ServiceContext>;
    /** The accepted requests by their controllers. */
    readonly #requests = new Map<AbortController, Request>();
    /** Resolve when accepted requests complete. */
    readonly #drained = Promise.withResolvers<void>();
    /** Resolve when draining and disposal finish. */
    readonly #stopped = Promise.withResolvers<void>();
    /** The shutdown. */
    #closing?: Promise<void>;
    /** Stop the controllers after draining. */
    readonly #reconciling = new AbortController();
    /** The running controllers. */
    readonly #controlled?: Promise<void>;
    /** The loop running the controllers, absent without controllers. */
    readonly #loop?: ControlLoop;

    /** Wait until the server stopped. */
    get stopped(): Promise<void> {
        return this.#stopped.promise;
    }

    /** Create the server. */
    private constructor(options: ServerOptions) {
        // reject missing enforcement
        for (const callback of ["authenticate", "authorizeHost"] as const) {
            if (typeof options[callback] !== "function") {
                throw new TypeError(`${callback} must be configured before starting a server`);
            }
        }

        // reject delays beyond the 32 bit timer
        if (
            !Number.isInteger(options.drainTimeout) ||
            options.drainTimeout <= 0 ||
            options.drainTimeout > MAX_TIMER_DELAY
        ) {
            throw new RangeError(
                "drain timeout must be a positive integer within the runtime timer limit",
            );
        }

        // require a policy for every permission
        Server.#requirePolicies(options.router, options.access);

        // create the handler
        this.#options = options;
        this.health = options.health;
        this.#handler = new ServiceHandler(options.router, {
            ...options,
            authorize: (call) => Server.#authorize(call, options),
        });

        // run the controllers
        const controllers = options.controllers ?? [];
        const access = options.access;
        if (controllers.length > 0 && access === undefined) {
            throw new TypeError("a service running controllers needs the database of its access");
        } else if (controllers.length > 0 && access !== undefined) {
            const loop = new ControlLoop(access.database, controllers, {
                report: reportReconciliation,
                ...(options.instance === undefined ? {} : { lease: { holder: options.instance } }),
                ...(options.alarm === undefined ? {} : { alarm: options.alarm }),
            });
            this.#loop = loop;
            this.#controlled = loop.run(this.#reconciling.signal);

            // stop serving once the controllers stop unasked, as a failure the host sees
            this.#controlled.catch((error: unknown) => {
                reportError(error);
                if (!this.#reconciling.signal.aborted) {
                    this.health.set("not-serving");
                }
            });
        }
    }

    /** Settle once no controller key is due now or reconciling. */
    idle(): Promise<void> {
        return this.#loop === undefined ? Promise.resolve() : this.#loop.idle();
    }

    /** Start the server. */
    static start(options: ServerOptions): Server {
        const server = new Server(options);
        server.health.set("serving");

        return server;
    }

    /** Serve a request. */
    async fetch(request: Request): Promise<Response> {
        // answer probes
        const probe = this.health.probe(request);
        if (probe) {
            return this.#headers(probe);
        }

        // refuse requests while draining
        if (this.#closing || this.health.status !== "serving") {
            return this.#headers(new Response(null, { status: 503 }));
        }

        // keep cancellation across the response
        const controller = new AbortController();
        const signal = AbortSignal.any([request.signal, controller.signal]);
        this.#requests.set(controller, request);
        try {
            // authenticate, then dispatch protocols or procedures
            const accepted = copyRequest(request, { signal });
            this.#requests.set(controller, accepted);
            const context = await Server.#authenticate(accepted, this.#options);
            signal.throwIfAborted();
            let response = await this.#options.route?.(accepted, context);
            if (response === undefined) {
                const result = await this.#handler.handle(accepted, { context });
                response = result.matched ? result.response : new Response(null, { status: 404 });
                response = attachBookmark(response, context.observed);
            }

            return this.#respond(this.#headers(response), controller, signal);
        } catch (error) {
            // release the request and report failures
            this.#finish(controller);
            if (signal.aborted) {
                throw error;
            }

            // map the failure to its HTTP response
            const failure = reportError(error);

            return this.#headers(Response.json(failure.toJSON(), { status: failure.status }));
        }
    }

    /** Refuse new work, drain requests and dispose resources, once. */
    close(): Promise<void> {
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Apply the response headers. */
    #headers(response: Response): Response {
        // keep responses without a header policy
        if (!this.#options.responseHeaders) {
            return response;
        }

        // copy the headers of immutable responses
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

    /** Authenticate a request and build its context. */
    static async #authenticate(request: Request, options: ServerOptions): Promise<ServiceContext> {
        // authenticate the request
        let authentication: Authentication | null = null;
        let authenticationError: Error | undefined;
        try {
            const authenticated = await options.authenticate(request);
            authenticated?.requireCurrent(
                options.audience,
                Date.now(),
                options.scope ?? authenticated.claims.scope,
            );
            authentication = authenticated;
        } catch (error) {
            authenticationError =
                error instanceof Error
                    ? error
                    : new ServiceError("UNAUTHORIZED", {
                          message: "authentication failed",
                          cause: error,
                      });
        }

        // digest the presented link secrets
        const presented = (request.headers.get(LINK_SECRET_HEADER) ?? "")
            .split(",")
            .map((secret) => secret.trim())
            .filter((secret) => secret.length > 0);
        const linkSecrets = await Promise.all(
            presented.map((secret) => LinkSecret.digest(secret)),
        ).catch((error: unknown) => {
            throw new ServiceError("UNAUTHORIZED", {
                message: "invalid link secret",
                cause: error,
            });
        });

        // scope the call to the service's scope or the caller's
        const scope = options.scope ?? authentication?.claims.scope;

        return new ServiceContext(request, {
            audience: options.audience,
            ...(scope === undefined ? {} : { scope }),
            authentication,
            resources: options.resources,
            ...(options.access === undefined ? {} : { access: options.access }),
            ...(authenticationError === undefined ? {} : { authenticationError }),
            linkSecrets,
            ...(options.clock === undefined ? {} : { clock: options.clock }),
        });
    }

    /** Authorize a call. */
    static async #authorize(
        call: ProcedureCall<ServiceContext>,
        options: ServerOptions,
    ): Promise<void> {
        // require identity on protected routes
        if (call.access.authentication !== "public") {
            call.context.requireAuthentication();
        }

        // renew the call's access
        call.context.authorization?.renew();

        // decide the permission on the call's target, which audits of a denial also read
        const permission = call.access.permission;
        if (permission !== null) {
            const access = options.access;
            const authorization = call.context.authorization;
            if (access?.target === undefined || authorization === undefined) {
                throw new TypeError(
                    `procedures requiring ${permission.name} need service access with targets`,
                );
            }
            const target = await access.target(call);
            call.context.target = target;
            await authorization.require(permission, target);
        }
        // check the caller in the service's scope
        else {
            Caller.delegation(call.context.access());
        }

        // let the host authorize the call
        await options.authorizeHost(call);
    }

    /** Keep a request open until its response body settles. */
    #respond(response: Response, controller: AbortController, signal: AbortSignal): Response {
        // finish a response without a body
        if (!response.body) {
            this.#finish(controller);

            return response;
        }

        // read the body through
        const reader = response.body.getReader();
        const finish = () => {
            signal.removeEventListener("abort", abort);
            this.#finish(controller);
        };

        // cancel the producer and release the request
        const abort = () => {
            void reader
                .cancel(signal.reason)
                .catch((error) => {
                    reportError(error);
                    responseError = error;
                })
                .finally(finish);
        };

        // forward cancellation and keep stream errors
        let responseError: unknown;
        signal.addEventListener("abort", abort, { once: true });
        if (signal.aborted) {
            abort();
        }

        // release the request when the stream settles
        const body = new ReadableStream<Uint8Array>({
            async pull(stream) {
                try {
                    // read the next chunk
                    const next = await reader.read();

                    // fail with the error a cancellation raised
                    if (responseError !== undefined) {
                        finish();
                        stream.error(responseError);

                        return;
                    }

                    // stop on cancellation
                    signal.throwIfAborted();

                    // close or forward the next chunk
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

    /** Release a request. */
    #finish(controller: AbortController): void {
        // wake shutdown after the last request
        this.#requests.delete(controller);
        if (this.#closing && !this.#requests.size) {
            this.#drained.resolve();
        }
    }

    /** Drain within the deadline. */
    async #close(): Promise<void> {
        // refuse new requests
        this.health.set("draining");
        if (!this.#requests.size) {
            this.#drained.resolve();
        }

        // report stopped after the requests finish
        const completion = this.#drained.promise.then(() => {
            this.health.set("stopped");
            this.#stopped.resolve();
        });

        // abort overdue requests at the deadline
        const deadline = Promise.withResolvers<never>();
        const timer = setTimeout(() => {
            const error = new DOMException("service drain deadline exceeded", "TimeoutError");
            for (const controller of this.#requests.keys()) {
                controller.abort(error);
            }

            deadline.reject(error);
        }, this.#options.drainTimeout);

        // wait for draining or the deadline
        try {
            await Promise.race([completion, deadline.promise]);
        } finally {
            // stop the controllers
            clearTimeout(timer);
            this.#reconciling.abort();
            await this.#controlled;
        }
    }

    /** Require a policy for every procedure permission. */
    static #requirePolicies(router: unknown, access: ServiceAccess | undefined): void {
        // check each procedure's permission
        if (isProcedure(router)) {
            const permission = ProcedureMeta.of(router).permission;
            if (permission === null) {
                return;
            }

            // require a target and a declaring policy
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
                Server.#requirePolicies(child, access);
            }
        }
    }
}

/** The options of a hosted service. */
export interface ServerOptions extends ServiceImplementation {
    /** The health. */
    health: Health;
    /** The receiving package. */
    audience: PackageId;
    /** The one scope the service serves, absent for regional services. */
    scope?: string;
    /** The installation's resource clients. */
    resources: ResourceContext;
    /** Verify credentials, returning null without a credential. */
    authenticate(request: Request): Promise<Authentication | null>;
    /** Enforce installation and host-only requirements. */
    authorizeHost(call: ProcedureCall<ServiceContext>): Promise<void>;
    /** The longest drain, in milliseconds. */
    drainTimeout: number;
    /** This instance's lease holder name. */
    instance?: string;
    /** Keep a wake-up for the controllers' earliest due key, so an evicted instance runs it. */
    alarm?: Alarm;
}

/** The procedures and enforcement of a service. */
export interface ServiceImplementation extends Omit<HandlerOptions<ServiceContext>, "health"> {
    /** The procedures. */
    router: Router<ServiceRouter, ServiceContext>;
    /** The headers of every response. */
    responseHeaders?: ConstructorParameters<typeof Headers>[0];
    /** The policies deciding permissions. */
    access?: ServiceAccess;
    /** Serve another HTTP protocol for an authenticated request, or return undefined. */
    route?(request: Request, context: ServiceContext): Promise<Response | undefined>;
    /** The service's controllers. */
    readonly controllers?: readonly Controller[];
    /** Read the current time calls run at, in UTC epoch milliseconds, the system clock by default. */
    readonly clock?: () => number;
}

/** Attach the request's watermarks to its response. */
function attachBookmark(response: Response, observed: Bookmark): Response {
    // keep a response without writes
    if (observed.watermarks.length === 0) {
        return response;
    }

    // add the bookmark header
    const headers = new Headers(response.headers);
    headers.set(BOOKMARK_HEADER, observed.format());

    return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers,
    });
}
