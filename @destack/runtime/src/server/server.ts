import type { Health } from "../health/health.ts";
import { RequestHandler, type HandlerOptions, type Router } from "./handler.ts";
import { isProcedure } from "@orpc/server";
import { ProcedureMeta } from "../procedure/procedure.ts";
import { type Authentication, Caller, Credential, LinkSecret } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { ProcedureRouter } from "../runtime/index.ts";
import { CallError, reportError, reportReconciliation } from "../error/index.ts";
import { BOOKMARK_HEADER, type Bookmark } from "../bookmark/index.ts";
import { LINK_SECRET_HEADER, RequestContext, type ServerAccess } from "./context.ts";
import type { ProcedureContext } from "./procedure-context.ts";
import { type Alarm, ControlLoop, type Controller } from "../control/index.ts";
import { copyRequest } from "../request/index.ts";
import { Clock, LONGEST_DELAY } from "@destack/environment";
import { until } from "../timer/index.ts";

/** The seconds a starting server delays a request, and a client then waits before retrying. */
const STARTING_RETRY_SECONDS = 1;

/** A hosted HTTP server. */
export class Server implements AsyncDisposable {
    /** The health. */
    readonly health: Health;
    /** The server options. */
    readonly #options: ServerOptions;
    /** The HTTP handler. */
    readonly #handler: RequestHandler<RequestContext>;
    /** The accepted requests by their controllers. */
    readonly #requests = new Map<AbortController, Request>();
    /** Resolve when accepted requests complete. */
    readonly #drained = Promise.withResolvers<void>();
    /** Resolve when draining and disposal finish. */
    readonly #stopped = Promise.withResolvers<void>();
    /** Resolve once the server serves. */
    readonly #serving = Promise.withResolvers<void>();
    /** The shutdown. */
    #closing?: Promise<void>;
    /** End the streams of accepted requests once draining starts. */
    readonly #draining = new AbortController();
    /** Stop the controllers after draining. */
    readonly #reconciling = new AbortController();
    /** The running controllers and the readiness wait. */
    #controlled?: Promise<void>;
    /** The running controllers, settled once they stop. */
    #running?: Promise<void>;
    /** The loop running the controllers, absent without controllers. */
    readonly #loop?: ControlLoop;

    /** Wait until the server stopped. */
    get stopped(): Promise<void> {
        return this.#stopped.promise;
    }

    /** Create the server. */
    private constructor(options: ServerOptions) {
        // reject missing enforcement
        for (const callback of ["authenticate", "authorizeMachine"] as const) {
            if (typeof options[callback] !== "function") {
                throw new TypeError(`${callback} must be configured before starting a server`);
            }
        }

        // reject delays beyond the 32 bit timer
        if (
            !Number.isInteger(options.drainTimeout) ||
            options.drainTimeout <= 0 ||
            options.drainTimeout > LONGEST_DELAY
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
        this.#handler = new RequestHandler(options.router, {
            ...options,
            authorize: (context) => Server.#authorize(context, options),
        });

        // run the controllers
        const controllers = options.controllers ?? [];
        const access = options.access;
        if (controllers.length > 0 && access === undefined) {
            throw new TypeError("a server with controllers needs its access database");
        } else if (controllers.length > 0 && access !== undefined) {
            const loop = new ControlLoop(access.database, controllers, {
                report: (controller, key, error) => {
                    // record the failure, and report it with the controller and the failed key
                    reportReconciliation(controller, key, error);
                    options.report?.(
                        new Error(`${controller.name} failed to reconcile ${key}`, {
                            cause: error,
                        }),
                    );
                },
                ...(options.instance === undefined ? {} : { lease: { holder: options.instance } }),
                ...(options.alarm === undefined ? {} : { alarm: options.alarm }),
                clock: options.clock,
            });
            this.#loop = loop;
        }
    }

    /** Settle once no controller key is due now or reconciling, or at a deadline. */
    idle(deadline?: number): Promise<void> {
        return this.#loop === undefined ? Promise.resolve() : this.#loop.idle(deadline);
    }

    /** Start the server: run the controllers maintaining its replicas, then serve once ready. */
    static start(options: ServerOptions): Server {
        // create the server, run its controllers and wait for its readiness in the background
        const server = new Server(options);
        server.#running = (async () => {
            // run the controllers once activated, as a workload's next start once it is current, unless stopped before
            const signal = server.#reconciling.signal;
            await Promise.race([options.activation, until(signal)]);
            if (!signal.aborted) {
                await server.#loop?.run(signal);
            }
        })();
        server.#controlled = Promise.all([server.#running, server.#serve()]).then(() => undefined);

        // stop serving once readiness or the controllers fail
        server.#controlled.catch((error: unknown) => {
            reportError(error);
            options.report?.(error);
            if (!server.#reconciling.signal.aborted) {
                server.health.set("not-serving");
            }
        });

        return server;
    }

    /** Stop the controllers at once, keeping the server's requests served until it drains. */
    async stopControllers(): Promise<void> {
        this.#reconciling.abort();
        await this.#running?.catch(() => undefined);
    }

    /** Serve at once without a readiness wait, else once ready. */
    async #serve(): Promise<void> {
        // serve at once without a readiness wait
        const signal = this.#reconciling.signal;
        const { ready } = this.#options;
        if (ready === undefined) {
            this.health.set("serving");
            this.#serving.resolve();
        }
        // wait for readiness, delaying requests while starting
        else {
            await ready(signal);
            if (signal.aborted) {
                return;
            }
            this.health.set("serving");
            this.#serving.resolve();
        }
    }

    /** Wait until the server serves, or for the retry window of a starting server. */
    async #awaitServing(): Promise<void> {
        // race serving with the window, then stop the timer
        let timer: ReturnType<typeof setTimeout> | undefined;
        const window = new Promise<void>((resolve) => {
            timer = setTimeout(resolve, STARTING_RETRY_SECONDS * 1000);
        });
        await Promise.race([this.#serving.promise, window]);
        clearTimeout(timer);
    }

    /** Serve a request, as a caller verified elsewhere in this process where one is given. */
    async fetch(request: Request, authentication?: Authentication): Promise<Response> {
        // answer probes
        const probe = this.health.probe(request);
        if (probe) {
            return this.#headers(probe);
        }

        // delay a request while starting, up to the retry window
        if (!this.#closing && this.health.status === "starting") {
            await this.#awaitServing();
        }

        // refuse requests still starting, asking to retry soon, and while draining
        if (!this.#closing && this.health.status === "starting") {
            return this.#headers(
                new Response(null, {
                    status: 503,
                    headers: { "retry-after": String(STARTING_RETRY_SECONDS) },
                }),
            );
        } else if (this.#closing || this.health.status !== "serving") {
            return this.#headers(new Response(null, { status: 503 }));
        }

        // carry cancellation across the response
        const controller = new AbortController();
        const signal = AbortSignal.any([request.signal, controller.signal]);
        this.#requests.set(controller, request);
        try {
            // dispatch authenticated protocols or procedures
            const accepted = copyRequest(request, { signal });
            this.#requests.set(controller, accepted);
            const context = await Server.#authenticate(
                accepted,
                this.#options,
                authentication,
                this.#draining.signal,
            );
            signal.throwIfAborted();
            const response = await this.#dispatch(accepted, context);

            return this.#respond(this.#headers(response), controller, signal);
        } catch (error) {
            // release the request and report failures, rethrowing those of a cancelled request
            const isCancelled = signal.aborted;
            this.#finish(controller);
            if (isCancelled) {
                throw error;
            }

            // map the failure to its HTTP response
            const failure = reportError(error);

            return this.#headers(Response.json(failure.toJSON(), { status: failure.status }));
        }
    }

    /** Dispatch an authenticated request to a protocol the server routes or to a procedure. */
    async #dispatch(request: Request, context: RequestContext): Promise<Response> {
        // serve a protocol the server routes itself
        const routed = await this.#options.fetch?.(request, context);
        if (routed !== undefined) {
            return routed;
        }

        // serve a procedure, answering the watermarks it observed
        const result = await this.#handler.handle(request, { context });
        const response = result.matched ? result.response : new Response(null, { status: 404 });

        return attachBookmark(response, context.observed);
    }

    /** Refuse new work, drain requests and dispose resources, once. */
    close(): Promise<void> {
        this.#closing ??= this.#close();

        return this.#closing;
    }

    /** Apply the response headers. */
    #headers(response: Response): Response {
        // leave responses without a header policy
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

    /** Close the server. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.close();
    }

    /** Authenticate a request, or take a caller verified elsewhere in this process. */
    static async #authenticate(
        request: Request,
        options: ServerOptions,
        given: Authentication | undefined,
        draining: AbortSignal,
    ): Promise<RequestContext> {
        // take the given caller, else authenticate the request, and require it current here
        let authenticated: Authentication | undefined;
        let authenticationError: Error | undefined;
        try {
            const verified =
                given ?? (await options.verify?.(request)) ?? (await options.authenticate(request));
            if (verified !== undefined) {
                const { credential } = verified;
                const scope = options.scope ?? credential.scope;
                Credential.requireCurrent(
                    credential,
                    options.package.id,
                    options.clock.now(),
                    scope,
                );
            }
            authenticated = verified;
        } catch (error) {
            authenticationError =
                error instanceof Error
                    ? error
                    : new CallError("UNAUTHORIZED", {
                          message: "authentication failed",
                          cause: error,
                      });
        }

        // digest the link secrets and scope the call
        const linkSecrets = await Server.#linkSecrets(request);
        const scope = options.scope ?? authenticated?.credential.scope;

        return new RequestContext(request, {
            audience: options.package.id,
            ...(scope === undefined ? {} : { scope }),
            ...(authenticated === undefined ? {} : { authentication: authenticated }),
            resources: options.resources,
            ...(options.access === undefined ? {} : { access: options.access }),
            ...(authenticationError === undefined ? {} : { authenticationError }),
            linkSecrets,
            clock: options.clock,
            draining,
        });
    }

    /** Digest the link secrets a request presents, refusing an invalid one. */
    static async #linkSecrets(request: Request) {
        const presented = (request.headers.get(LINK_SECRET_HEADER) ?? "")
            .split(",")
            .map((secret) => secret.trim())
            .filter((secret) => secret.length > 0);

        return Promise.all(presented.map((secret) => LinkSecret.digest(secret))).catch(
            (error: unknown) => {
                throw new CallError("UNAUTHORIZED", {
                    message: "invalid link secret",
                    cause: error,
                });
            },
        );
    }

    /** Authorize a call. */
    static async #authorize(
        context: ProcedureContext<RequestContext>,
        options: ServerOptions,
    ): Promise<void> {
        // require an identified caller on protected routes
        if (
            context.access.authentication !== "public" &&
            Caller.isAnonymous(context.request.caller())
        ) {
            throw new CallError("UNAUTHORIZED", { message: "missing caller credential" });
        }

        // renew the call's access
        context.request.authorization?.renew();

        // allow anyone, and leave each row to the object store, refusing a credential that failed
        const permission = context.access.permission;
        if (permission === "anyone" || permission === "rows" || "rows" in permission) {
            Caller.requireChain(context.request.caller());
        }
        // allow principals of the declared kinds calling as themselves
        else if ("principals" in permission) {
            const caller = context.request.caller();
            const sender = Caller.sender(caller);
            const isDeclared = permission.principals.some(
                (kind) => kind.packageId === sender.packageId && kind.type === sender.type,
            );
            if (!isDeclared) {
                throw new CallError("FORBIDDEN", {
                    message: `${sender.type} callers cannot call ${context.path.join(".")}`,
                });
            }
            Caller.requireChain(caller);
        }
        // decide the permission on the call's target
        else {
            const access = options.access;
            const authorization = context.request.authorization;
            if (access?.target === undefined || authorization === undefined) {
                throw new TypeError(
                    `procedures requiring ${permission.name} need server access with targets`,
                );
            }
            const target = await access.target(context);
            context.request.target = target;
            await authorization.require(permission, target);
        }

        // let the host authorize the call
        await options.authorizeMachine(context);
    }

    /** Leave a request open until its response body settles. */
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

        // forward cancellation and stream errors
        let responseError: unknown;
        signal.addEventListener("abort", abort, { once: true });
        if (signal.aborted) {
            abort();
        }

        // release the request when the stream settles
        const body = relayBody(reader, signal, finish, () => responseError);

        return new Response(body, {
            status: response.status,
            statusText: response.statusText,
            headers: response.headers,
        });
    }

    /** Release a request. */
    #finish(controller: AbortController): void {
        // end the request's signal, and what waits on it
        controller.abort(new DOMException("the request finished", "AbortError"));

        // wake shutdown after the last request
        this.#requests.delete(controller);
        if (this.#closing && !this.#requests.size) {
            this.#drained.resolve();
        }
    }

    /** Drain within the deadline. */
    async #close(): Promise<void> {
        // refuse new requests and end the open streams at their next consistent point
        this.health.set("draining");
        this.#draining.abort(new DOMException("the server drains", "AbortError"));
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
            const error = new DOMException("server drain deadline exceeded", "TimeoutError");
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
    static #requirePolicies(router: unknown, access: ServerAccess | undefined): void {
        // check each procedure's permission
        if (isProcedure(router)) {
            const permission = ProcedureMeta.of(router).permission;
            if (typeof permission === "string" || !("name" in permission)) {
                return;
            }

            // require a target and a declaring policy
            if (access?.target === undefined) {
                throw new TypeError(
                    `procedures requiring ${permission.name} need server access with targets`,
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

/** The options of a hosted server. */
export interface ServerOptions extends PackageImplementation {
    /** The health. */
    health: Health;
    /** The clock calls run at. */
    readonly clock: Clock;
    /** The one scope the server serves, absent for a server of every scope. */
    scope?: string;
    /** The installation's resource clients. */
    resources: ResourceContext;
    /** Verify credentials as the host after the package, undefined without a credential. */
    readonly authenticate: (request: Request) => Promise<Authentication | undefined>;
    /** Enforce installation and host-only requirements. */
    readonly authorizeMachine: (call: ProcedureContext<RequestContext>) => Promise<void>;
    /** The longest drain, in milliseconds. */
    drainTimeout: number;
    /** This instance's lease holder name. */
    instance?: string;
    /** Set a wake-up for the controllers' earliest due key, so an evicted instance runs it. */
    alarm?: Alarm;
    /** Report a controller's failure to the host beside its telemetry. */
    readonly report?: (error: unknown) => void;
}

/** The procedures and enforcement of a package. */
export interface PackageImplementation extends Omit<HandlerOptions<RequestContext>, "health"> {
    /** The procedures. */
    router: Router<ProcedureRouter, RequestContext>;
    /** The headers of every response. */
    responseHeaders?: ConstructorParameters<typeof Headers>[0];
    /** The policies deciding permissions. */
    access?: ServerAccess;
    /** Serve another HTTP protocol for an authenticated request, or return undefined. */
    readonly fetch?: (request: Request, context: RequestContext) => Promise<Response | undefined>;
    /** The package's controllers. */
    readonly controllers?: readonly Controller[];
    /** Verify a credential whose keys the package has before the host, undefined for another. */
    readonly verify?: (request: Request) => Promise<Authentication | undefined>;
    /** Wait until the package can answer before it serves. */
    readonly ready?: (signal: AbortSignal) => Promise<void>;
    /** Start the controllers once this settles, at once when absent. */
    readonly activation?: Promise<void>;
}

/** Attach the request's watermarks to its response. */
function attachBookmark(response: Response, observed: Bookmark): Response {
    // leave a response without writes
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

/** Relay a response body chunk by chunk. */
function relayBody(
    reader: Pick<ReadableStreamDefaultReader<Uint8Array>, "read" | "cancel">,
    signal: AbortSignal,
    finish: () => void,
    cancellationError: () => unknown,
): ReadableStream<Uint8Array> {
    return new ReadableStream<Uint8Array>({
        async pull(stream) {
            try {
                // read the next chunk
                const next = await reader.read();

                // fail with the error a cancellation raised
                const error = cancellationError();
                if (error !== undefined) {
                    finish();
                    stream.error(error);

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
}
