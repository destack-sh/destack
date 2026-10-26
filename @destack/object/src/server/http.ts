import type { DigestCrypto } from "@destack/schema";
import type { Health } from "./health.ts";
import { ObjectError } from "../error/error.ts";
import { RequestHandler, type HandlerOptions, type Router } from "./handler.ts";
import { isProcedure } from "@orpc/server";
import { ProcedureMetadata } from "../procedure/procedure.ts";
import { type Authentication, Caller, Credential, LinkSecret } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { ProcedureRouter } from "../procedure/index.ts";

import { refusal, reportError } from "../error/report.ts";
import { BOOKMARK_HEADER, type Bookmark } from "../procedure/bookmark.ts";
import { LINK_SECRET_HEADER, RequestContext, type ServerAccess } from "./context.ts";
import type { ProcedureContext } from "./procedure.ts";
import { type Alarm, ControlLoop, type Controller } from "@destack/db";
import { copyRequest, type Environment, LONGEST_DELAY, until } from "@destack/environment";

import type {} from "@destack/package/import-meta";

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
    readonly #readiness = Promise.withResolvers<void>();
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
    /** The lapse of each credential in use by its key and expiry, one timer per credential. */
    readonly #lapses = new Map<string, AbortSignal>();
    /** The loop running the controllers syncing the replicas, absent without them. */
    readonly #syncing: ControlLoop | undefined;
    /** The loop running the other controllers once the replicas are synced, absent without them. */
    readonly #loop: ControlLoop | undefined;

    /** Wait until the server stopped. */
    get stopped(): Promise<void> {
        return this.#stopped.promise;
    }

    /** Create the server. */
    private constructor(options: ServerOptions) {
        // leave the refusal of a server stopped before it served to those waiting for it
        this.#readiness.promise.catch(() => undefined);

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

        // run the controllers syncing the replicas, and the others once they are synced
        this.#syncing = Server.#loopOf(options.syncing ?? [], options);
        this.#loop = Server.#loopOf(options.controllers ?? [], options);
    }

    /** Create the loop of some controllers over the access database, absent for none. */
    static #loopOf(
        controllers: readonly Controller[],
        options: ServerOptions,
    ): ControlLoop | undefined {
        // run no loop without controllers, and refuse controllers without an access database
        const { access } = options;
        if (controllers.length === 0) {
            return undefined;
        } else if (access === undefined) {
            throw new TypeError("a server with controllers needs an access database");
        }

        return new ControlLoop(access.database, controllers, {
            report: (controller, key, error) => {
                // report the failure with the controller and the failed key
                options.report?.(
                    new Error(`${controller.name} failed to reconcile ${key}`, { cause: error }),
                );
            },
            ...(options.instance === undefined ? {} : { lease: { holder: options.instance } }),
            ...(options.alarm === undefined ? {} : { alarm: options.alarm }),
            clock: options.environment.clock,
        });
    }

    /** Settle once the server serves, its readiness met. */
    ready(): Promise<void> {
        return this.#readiness.promise;
    }

    /** Settle once no controller key is due now or reconciling, or at a deadline. */
    idle(deadline?: number): Promise<void> {
        return Promise.all([this.#syncing?.idle(deadline), this.#loop?.idle(deadline)]).then(
            () => undefined,
        );
    }

    /** Start the server: run the controllers maintaining its replicas, then serve once ready. */
    static start(options: ServerOptions): Server {
        // create the server, run its controllers and wait for its readiness in the background
        const server = new Server(options);
        server.#running = (async () => {
            // run the controllers once activated, as a workload's next start once it is current, unless stopped before
            const signal = server.#reconciling.signal;
            await options.activation?.(signal);
            if (!signal.aborted) {
                await Promise.all([server.#syncing?.run(signal), server.#runOnceSynced(signal)]);
            }
        })();
        server.#controlled = Promise.all([server.#running, server.#serve()]).then(() => undefined);

        // stop serving once readiness or the controllers fail
        server.#controlled.catch((error: unknown) => {
            reportError(error, options.environment.telemetry);
            options.report?.(error);
            if (!server.#reconciling.signal.aborted) {
                server.health.set("not-serving");
            }
        });

        return server;
    }

    /** Run the other controllers once the replicas are synced, unless stopped first. */
    async #runOnceSynced(signal: AbortSignal): Promise<void> {
        await this.#options.synced?.(signal);
        if (!signal.aborted) {
            await this.#loop?.run(signal);
        }
    }

    /** Stop the controllers at once, while the server serves its requests until it drains. */
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
            this.#readiness.resolve();
        }
        // wait for readiness, delaying requests while starting
        else {
            try {
                await ready(signal);
            } catch (error) {
                // fail with the readiness failure, unless the stop that aborts it ended the wait
                if (!signal.aborted) {
                    throw error;
                }
            }

            // refuse waiting for a server stopped before it served, else serve
            if (signal.aborted) {
                this.#readiness.reject(
                    ObjectError.of(
                        "SERVER_STOPPED",
                        "the server stopped before it started serving",
                    ),
                );
            } else {
                this.health.set("serving");
                this.#readiness.resolve();
            }
        }
    }

    /** Wait until the server serves, or for the retry window of a starting server. */
    async #awaitServing(): Promise<void> {
        // race serving with the window on the server's clock
        const window = this.#options.environment.clock.timeout(STARTING_RETRY_SECONDS * 1000);
        await Promise.race([this.#readiness.promise, until(window)]);
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

        // refuse requests still starting, asking to retry soon, and while draining, stopped or failed
        if (!this.#closing && this.health.status === "starting") {
            const starting = ObjectError.of("SERVER_STARTING", "the server is starting");
            const refused = refusal(starting, this.#options.environment.telemetry);
            refused.headers.set("retry-after", String(STARTING_RETRY_SECONDS));

            return this.#headers(refused);
        } else if (this.#closing || this.health.status !== "serving") {
            return this.#headers(refusal(this.#unavailable(), this.#options.environment.telemetry));
        }

        // pass cancellation across the response
        const controller = new AbortController();
        const signal = AbortSignal.any([request.signal, controller.signal]);
        this.#requests.set(controller, request);
        try {
            // dispatch authenticated protocols or procedures
            const accepted = copyRequest(request, { signal });
            this.#requests.set(controller, accepted);
            const context = await this.#authenticate(accepted, authentication);
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
            const failure = reportError(error, this.#options.environment.telemetry);

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

    /** Describe why a server past starting refuses a request: draining, stopped or failed. */
    #unavailable(): Error {
        const { status } = this.health;
        if (status === "stopped") {
            return ObjectError.of("SERVER_STOPPED", "the server stopped");
        } else if (status === "not-serving") {
            return ObjectError.of(
                "SERVER_NOT_SERVING",
                "the server stopped serving after a failure",
            );
        } else {
            return ObjectError.of("SERVER_DRAINING", "the server is draining");
        }
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

    /** Scope a call: the credential's scope where the server serves it, else the server's own. */
    static #scopeOf(
        scopes: readonly string[] | undefined,
        bound: string | undefined,
    ): string | undefined {
        return scopes === undefined || (bound !== undefined && scopes.includes(bound))
            ? bound
            : scopes[0];
    }

    /** Authenticate a request, or take a caller verified elsewhere in this process. */
    async #authenticate(
        request: Request,
        given: Authentication | undefined,
    ): Promise<RequestContext> {
        // read the server's options and its drain
        const options = this.#options;
        const draining = this.#draining.signal;

        // take the given caller, else authenticate the request, and require it current here
        let authenticated: Authentication | undefined;
        let authenticationError: Error | undefined;
        try {
            const verified =
                given ?? (await options.verify?.(request)) ?? (await options.authenticate(request));
            if (verified !== undefined) {
                const { credential } = verified;
                const scope = Server.#scopeOf(options.scopes, credential.scope);
                Credential.requireCurrent(
                    credential,
                    options.package.id,
                    options.environment.clock.now(),
                    scope,
                );
            }
            authenticated = verified;
        } catch (error) {
            authenticationError =
                error instanceof Error
                    ? error
                    : ObjectError.of("UNAUTHENTICATED", "authentication failed", { cause: error });
        }

        // digest the link secrets and scope the call
        const linkSecrets = await Server.#linkSecrets(request, this.#options.environment.crypto);
        const scope = Server.#scopeOf(options.scopes, authenticated?.credential.scope);

        return new RequestContext(request, {
            audience: options.package.id,
            ...(scope === undefined ? {} : { scope }),
            ...(authenticated === undefined ? {} : { authentication: authenticated }),
            resources: options.resources,
            ...(options.access === undefined ? {} : { access: options.access }),
            ...(authenticationError === undefined ? {} : { authenticationError }),
            linkSecrets,
            environment: options.environment,
            draining,
            ...(authenticated === undefined
                ? {}
                : { lapse: this.#lapseOf(authenticated.credential) }),
        });
    }

    /** Read the signal a credential's lapse aborts, shared by its requests until it lapses. */
    #lapseOf(credential: Credential): AbortSignal {
        // share the lapse of a credential seen before
        const key = `${Credential.key(credential)} ${credential.expiresAt}`;
        const known = this.#lapses.get(key);
        if (known !== undefined) {
            return known;
        }

        // arm one timer for its lapse, forgotten once it lapses
        const lapse = RequestContext.lapse(
            credential,
            this.#options.environment.clock,
            this.#draining.signal,
        );
        this.#lapses.set(key, lapse);
        lapse.addEventListener("abort", () => this.#lapses.delete(key), { once: true });

        return lapse;
    }

    /** Digest the link secrets a request presents, refusing an invalid one. */
    static async #linkSecrets(request: Request, crypto: DigestCrypto) {
        const presented = (request.headers.get(LINK_SECRET_HEADER) ?? "")
            .split(",")
            .map((secret) => secret.trim())
            .filter((secret) => secret.length > 0);

        return Promise.all(presented.map((secret) => LinkSecret.digest(secret, crypto))).catch(
            (error: unknown) => {
                throw ObjectError.of("LINK_SECRET_INVALID", "invalid link secret", {
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
            throw ObjectError.of("CREDENTIAL_MISSING", "missing caller credential");
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
                throw ObjectError.of(
                    "CALLER_KIND_REFUSED",
                    `${sender.type} callers cannot call ${context.path.join(".")}`,
                );
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
                    reportError(error, this.#options.environment.telemetry);
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

        // abort overdue requests at the deadline on the server's clock
        const deadline = Promise.withResolvers<never>();
        const expiry = this.#options.environment.clock.timeout(this.#options.drainTimeout);
        const expire = () => {
            const error = new DOMException("server drain deadline exceeded", "TimeoutError");
            for (const controller of this.#requests.keys()) {
                controller.abort(error);
            }

            deadline.reject(error);
        };
        expiry.addEventListener("abort", expire, { once: true });

        // wait for draining or the deadline
        try {
            await Promise.race([completion, deadline.promise]);
        } finally {
            // stop the deadline and the controllers
            expiry.removeEventListener("abort", expire);
            this.#reconciling.abort();
            await this.#controlled;
        }
    }

    /** Require a policy for every procedure permission. */
    static #requirePolicies(router: unknown, access: ServerAccess | undefined): void {
        // check each procedure's permission
        if (isProcedure(router)) {
            const permission = ProcedureMetadata.of(router).permission;
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
export interface ServerOptions extends PackageHandler {
    /** The health. */
    health: Health;
    /** The environment calls run in: their clock, telemetry and effects. */
    readonly environment: Environment;
    /** The scopes the server serves, its own first, absent for a server of every scope. */
    scopes?: readonly string[];
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
export interface PackageHandler extends Omit<
    HandlerOptions<RequestContext>,
    "health" | "environment"
> {
    /** The procedures. */
    router: Router<ProcedureRouter, RequestContext>;
    /** The headers of every response. */
    responseHeaders?: ConstructorParameters<typeof Headers>[0];
    /** The policies deciding permissions. */
    access?: ServerAccess;
    /** Serve another HTTP protocol for an authenticated request, or return undefined. */
    readonly fetch?: (request: Request, context: RequestContext) => Promise<Response | undefined>;
    /** The package's controllers, started once its replicas are synced. */
    readonly controllers?: readonly Controller[];
    /** The controllers syncing the package's replicas, started at once. */
    readonly syncing?: readonly Controller[];
    /** Wait until the package's replicas are synced, at once when absent. */
    readonly synced?: (signal: AbortSignal) => Promise<void>;
    /** Verify a credential whose keys the package has before the host, undefined for another. */
    readonly verify?: (request: Request) => Promise<Authentication | undefined>;
    /** Wait until the package can answer before it serves. */
    readonly ready?: (signal: AbortSignal) => Promise<void>;
    /** Activate the package before its controllers run, stopping once the signal aborts. */
    readonly activation?: (signal: AbortSignal) => Promise<void>;
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
