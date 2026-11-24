import { type DigestCrypto, Uuid } from "@destack/schema";
import type { PackageId } from "@destack/package";
import { type Health, STARTING_RETRY_MILLISECONDS } from "./health.ts";
import { ObjectError } from "../error/error.ts";
import { RequestHandler, type HandlerOptions, type Router } from "./handler.ts";
import { isProcedure } from "@orpc/server";
import { ProcedureMetadata } from "../procedure/procedure.ts";
import { type Authentication, Caller, Credential, LinkSecret } from "@destack/access";
import type { ResourceContext } from "@destack/resource/context";
import type { ProcedureRouter } from "../procedure/index.ts";

import { refusal, reportError } from "../error/report.ts";
import { RetryAfter } from "../error/detail.ts";
import {
    LINK_SECRET_HEADER,
    RequestContext,
    type RequestContextOptions,
    type ServerAccess,
} from "./context.ts";
import { ObjectConnectionServer, type ObjectConnectionAttachment } from "./connection.ts";
import type { StandardRPCHandler } from "@orpc/server/standard";
import {
    CONNECTION_PING,
    CONNECTION_PONG,
    CONNECTION_PROTOCOL,
    CONNECTION_SCOPE_PARAMETER,
    ObjectConnection,
} from "../replica/connection.ts";
import type { Hibernation, SocketHandler } from "../workload/socket.ts";
import type { ProcedureContext } from "./procedure.ts";
import { copyRequest, type Environment, LONGEST_DELAY } from "@destack/environment";

import type {} from "@destack/package/import-meta";

/** A hosted HTTP server. */
export class Server implements AsyncDisposable {
    /** The health. */
    readonly health: Health;
    /** The server options, which an update replaces. */
    #options: ServerOptions;
    /** The HTTP handler, which an update replaces. */
    #handler: RequestHandler<RequestContext>;
    /** The accepted requests by their controllers. */
    readonly #requests = new Map<AbortController, Request>();
    /** Resolve when accepted requests complete. */
    readonly #drained = Promise.withResolvers<void>();
    /** Resolve when draining and disposal finish. */
    readonly #stopped = Promise.withResolvers<void>();
    /** The shutdown. */
    #closing?: Promise<void>;
    /** End the streams of accepted requests once draining starts. */
    readonly #draining = new AbortController();
    /** Abort the readiness wait once the server closes. */
    readonly #starting = new AbortController();
    /** The readiness wait. */
    #readiness?: Promise<void>;
    /** The lapse of each credential in use by its key and expiry, one timer per credential. */
    readonly #lapses = new Map<string, AbortSignal>();
    /** The open connections by their sockets' identifiers, each with its upgrade's request. */
    readonly #connections = new Map<
        string,
        { readonly connection: ObjectConnectionServer; readonly controller: AbortController }
    >();

    /** Wait until the server stopped. */
    get stopped(): Promise<void> {
        return this.#stopped.promise;
    }

    /** Create the server. */
    private constructor(options: ServerOptions) {
        this.#options = options;
        this.health = options.health;
        this.#handler = Server.#handlerOf(options);
    }

    /** Check a server's options and build its handler. */
    static #handlerOf(options: ServerOptions): RequestHandler<RequestContext> {
        // reject missing enforcement
        for (const callback of ["authenticate", "authorizeMachine"] as const) {
            if (typeof options[callback] !== "function") {
                throw new TypeError(`'${callback}' must be configured before starting a server`);
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

        return new RequestHandler(options.router, {
            ...options,
            authorize: (context) => Server.#authorize(context, options),
        });
    }

    /** Serve another handler of the same package in place, its connections calling the new one. */
    update(options: ServerOptions): void {
        // require the same package
        if (options.package.id !== this.#options.package.id) {
            throw new TypeError(
                `a server of package '${this.#options.package.id}' serves no other package`,
            );
        }

        // take the new handler and options
        this.#handler = Server.#handlerOf(options);
        this.#options = options;

        // end the connections of a handler that serves no store any more
        if (this.#handler.connections === undefined) {
            for (const id of this.#connections.keys()) {
                this.#end(id).catch((error: unknown) =>
                    reportError(error, this.#options.environment.telemetry),
                );
            }
        }
    }

    /** Read the store procedures the connections call, refusing a package serving no store. */
    #procedures(): StandardRPCHandler<RequestContext> {
        const handler = this.#handler.connections;
        if (handler === undefined) {
            throw ObjectError.of(
                "CONNECTION_NOT_FOUND",
                `connection not found: package '${this.#options.package.id}' serves no store`,
            );
        }

        return handler;
    }

    /** Settle once the server serves, refusing once it fails or stops first. */
    ready(): Promise<void> {
        return this.health.waitUp();
    }

    /** Start the server, serving once ready. */
    static start(options: ServerOptions): Server {
        // create the server and wait for its readiness in the background
        const server = new Server(options);
        server.#readiness = server.#serve();

        // stop serving once readiness fails, a stop's own abort ending it quietly
        server.#readiness.catch((error: unknown) => {
            if (error === server.#starting.signal.reason) {
                return;
            }
            reportError(error, options.environment.telemetry);
            if (!server.#starting.signal.aborted) {
                const message = error instanceof Error ? error.message : String(error);
                server.health.set("down", `the server failed: ${message}`);
            }
        });

        return server;
    }

    /** Serve at once without a readiness wait, else once ready. */
    async #serve(): Promise<void> {
        // serve at once without a readiness wait
        const signal = this.#starting.signal;
        const { ready } = this.#options;
        if (ready === undefined) {
            this.health.set("up", "the server serves without a readiness wait");
        }
        // wait for readiness, delaying requests while starting
        else {
            try {
                await ready(signal, this.health);
            } catch (error) {
                // fail with the readiness failure, unless the stop that aborts it ended the wait
                if (!signal.aborted) {
                    throw error;
                }
            }

            // serve, unless the stop that aborted the wait moves the health on
            if (!signal.aborted) {
                this.health.set("up", "the server's readiness is met");
            }
        }
    }

    /** Wait until the server serves, or for the retry window of a starting server. */
    async #waitUp(): Promise<void> {
        // wait for serving up to the window on the server's clock, the refusal answering any other end
        const window = this.#options.environment.clock.timeout(STARTING_RETRY_MILLISECONDS);
        await this.health.waitUp(window).catch(() => undefined);
    }

    /** Serve a request, each of its calls as the caller a voucher in this process vouches for. */
    async fetch(request: Request, voucher?: Voucher): Promise<Response> {
        // answer probes
        const probe = this.health.probe(request);
        if (probe) {
            return this.#headers(probe);
        }

        // delay a request while starting, up to the retry window
        if (this.health.status === "starting") {
            await this.#waitUp();
        }

        // refuse a request while not serving
        const refused = this.health.refusal();
        if (refused !== undefined) {
            return this.#headers(refusal(refused, this.#options.environment.telemetry));
        }

        // pass cancellation across the response
        const controller = new AbortController();
        const signal = AbortSignal.any([request.signal, controller.signal]);
        this.#requests.set(controller, request);
        try {
            // dispatch authenticated protocols or procedures
            const accepted = copyRequest(request, { signal });
            this.#requests.set(controller, accepted);
            const verified = await this.#authenticate(accepted, voucher);
            signal.throwIfAborted();

            // accept a connection, which stays open as one request until its socket closes
            if (ObjectConnection.isUpgrade(accepted)) {
                return this.#headers(this.#connect(accepted, verified, controller, voucher));
            }

            // dispatch a request
            const context = new RequestContext(accepted, verified);
            const response = await this.#dispatch(accepted, context);

            return this.#respond(this.#headers(response), controller, signal);
        } catch (error) {
            // release the request and report failures, rethrowing those of a cancelled request
            const isCancelled = signal.aborted;
            this.#finish(controller);
            if (isCancelled) {
                throw error;
            }

            // answer the failure as a refusal, with when to retry
            return this.#headers(refusal(error, this.#options.environment.telemetry));
        }
    }

    /** Take the connections' frames and closes from the host that keeps their sockets. */
    get sockets(): SocketHandler {
        return {
            message: async (socket, message) => {
                const connection = this.#connections.get(socket.id)?.connection;
                // close a socket the server no longer serves, as after hibernation, for its client to resume
                if (connection === undefined) {
                    socket.close();
                }
                // refuse a binary frame
                else if (typeof message !== "string") {
                    throw ObjectError.of(
                        "FRAME_INVALID",
                        "frame is invalid: a connection takes JSON text frames",
                    );
                }
                // start the frame's call
                else {
                    connection.message(message);
                }
            },
            close: (socket) => this.#end(socket.id),
        };
    }

    /** End a connection once: close its socket, wait for its calls, then release its request. */
    async #end(id: string): Promise<void> {
        // end a connection still open
        const open = this.#connections.get(id);
        if (open === undefined) {
            return;
        }
        this.#connections.delete(id);

        // close its socket, let its calls and follows finish, then release the upgrade's request from the drain
        open.connection.socket.close();
        await open.connection.close();
        this.#finish(open.controller);
    }

    /** Accept a connection's upgrade, closing it as the server drains. */
    #connect(
        upgrade: Request,
        verified: RequestContextOptions,
        controller: AbortController,
        voucher: Voucher | undefined,
    ): Response {
        // refuse a connection once the server drains, as its drain would never end it
        const refused = this.health.refusal();
        if (refused !== undefined) {
            throw refused;
        }

        // require procedures to connect to and a host that keeps sockets
        const { hibernation } = this.#options;
        this.#procedures();
        if (hibernation === undefined) {
            throw ObjectError.of(
                "CONNECTION_UNAVAILABLE",
                "connection is unavailable: the server's host keeps no sockets",
            );
        }

        // refuse a connection confined to a scope outside its credential's, then serve its calls
        Server.#confine(upgrade, verified);
        const id = Uuid.v7(this.#options.environment);
        const attachment: ObjectConnectionAttachment = {
            connection: this.#options.package.id,
        };
        const socket = hibernation.socket({ id, attachment });
        const connection = new ObjectConnectionServer(
            socket,
            upgrade,
            () => this.#procedures(),
            this.#options.environment.telemetry,
            async (request) =>
                // vouch for an in-process caller again at each call, and verify any other call's own credential
                Server.#confine(upgrade, await this.#authenticate(request, voucher)),
        );
        this.#connections.set(id, { connection, controller });

        // end the connection as the server drains, its follows ending at their sessions' lapses
        this.#draining.signal.addEventListener("abort", () => void this.#end(id), { once: true });

        // answer the client's liveness pings at the host, without waking the server
        return hibernation.accept({
            id,
            attachment,
            protocol: CONNECTION_PROTOCOL,
            respond: { request: CONNECTION_PING, response: CONNECTION_PONG },
        });
    }

    /** Confine a connection to the scope its upgrade names, inside its credential's own scope. */
    static #confine(upgrade: Request, verified: RequestContextOptions): RequestContextOptions {
        // take the credential's scope without a named one
        const confined = new URL(upgrade.url).searchParams.get(CONNECTION_SCOPE_PARAMETER);
        if (confined === null) {
            return verified;
        }
        // refuse a scope outside the credential's
        else if (verified.scope !== undefined && verified.scope !== confined) {
            throw ObjectError.of(
                "SCOPE_OUTSIDE_REQUEST_NOT_FOUND",
                `scope '${confined}' in the request's scope not found`,
            );
        }

        return { ...verified, scope: confined };
    }

    /** Dispatch an authenticated request to a protocol the server routes or to a procedure. */
    async #dispatch(request: Request, context: RequestContext): Promise<Response> {
        // serve a protocol the server routes itself
        const routed = await this.#options.fetch?.(request, context);
        if (routed !== undefined) {
            return routed;
        }

        // serve a procedure, answering the watermarks it observed, refusing a route none serves
        const result = await this.#handler.handle(request, { context });
        if (!result.matched) {
            const { pathname } = new URL(request.url);
            throw ObjectError.of(
                "ROUTE_NOT_FOUND",
                `route '${request.method}' '${pathname}' not found`,
            );
        }

        return context.observed.attach(await RetryAfter.attach(result.response));
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
        voucher: Voucher | undefined,
    ): Promise<RequestContextOptions> {
        // read the server's options and its drain
        const options = this.#options;
        const draining = this.#draining.signal;

        // take the caller a voucher in this process vouches for, else verify the call's own credential, and require it current for the package or a protocol it implements
        let audience: PackageId = options.package.id;
        let authenticated: Authentication | undefined;
        let authenticationError: Error | undefined;
        try {
            const given = await voucher?.(request);
            const verified = given ?? (await options.authenticate(request));
            if (verified !== undefined) {
                const { credential } = verified;
                const scope = Server.#scopeOf(options.scopes, credential.scope);
                if (options.protocols?.has(credential.audience) === true) {
                    audience = credential.audience;
                }
                Credential.requireCurrent(
                    credential,
                    audience,
                    options.environment.clock.now(),
                    scope,
                );
            }
            authenticated = verified;
        } catch (error) {
            authenticationError =
                error instanceof Error
                    ? error
                    : ObjectError.of(
                          "CALLER_UNAUTHENTICATED",
                          "authentication required: authentication failed",
                          { cause: error },
                      );
        }

        // digest the link secrets and scope the call
        const linkSecrets = await Server.#linkSecrets(request, this.#options.environment.crypto);
        const scope = Server.#scopeOf(options.scopes, authenticated?.credential.scope);

        return {
            audience,
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
        };
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
                throw ObjectError.of(
                    "LINK_SECRET_UNAUTHENTICATED",
                    "authentication required: link secret does not match",
                    {
                        cause: error,
                    },
                );
            },
        );
    }

    /** Authorize a call. */
    static async #authorize(
        context: ProcedureContext<RequestContext>,
        options: ServerOptions,
    ): Promise<void> {
        // require a credential on protected routes, refusing a failed one
        if (context.access.authentication !== "public") {
            context.request.credential();
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
                    "CALLER_KIND_DENIED",
                    `'${context.path.join(".")}' is forbidden: ${sender.type} callers cannot call it`,
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
                    `procedures requiring '${permission.name}' need server access with targets`,
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
        this.health.set("draining", "the server drains its requests");
        this.#draining.abort(new DOMException("the server drains", "AbortError"));
        if (!this.#requests.size) {
            this.#drained.resolve();
        }

        // report stopped after the requests finish
        const completion = this.#drained.promise.then(() => {
            this.health.set("stopped", "the server drained and stopped");
            this.#stopped.resolve();
        });

        // abort overdue requests at the deadline on the server's clock, a deadline the drain disarms
        const disarm = new AbortController();
        const deadline = this.#options.environment.clock
            .sleep(this.#options.drainTimeout, disarm.signal)
            .then(() => {
                // list the requests still open
                const open = [...this.#requests.values()].map((request) => {
                    const url = new URL(request.url);

                    return `${request.method} ${url.host}${url.pathname}`;
                });
                const error = new DOMException(
                    `server drain deadline exceeded with requests open: ${open.join(", ")}`,
                    "TimeoutError",
                );

                // abort them with the deadline's error, then fail the drain with it
                for (const controller of this.#requests.keys()) {
                    controller.abort(error);
                }
                throw error;
            });

        // wait for draining or the deadline
        try {
            await Promise.race([completion, deadline]);
        } finally {
            // stop the deadline and the readiness wait
            disarm.abort();
            deadline.catch(() => undefined);
            this.#starting.abort();

            // wait for the readiness wait to end, whose failure its start reported already
            await this.#readiness?.catch(() => undefined);
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
                    `procedures requiring '${permission.name}' need server access with targets`,
                );
            }
            if (
                !Object.hasOwn(
                    access.authorizer.policy(permission).definition.permissions,
                    permission.name,
                )
            ) {
                throw new TypeError(`no policy declares '${permission.type}.${permission.name}'`);
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
    /** The host's keeping of the connections' sockets, absent for a host without sockets. */
    readonly hibernation?: Hibernation;
}

/** Vouch for an in-process caller at one call, absent for an anonymous one. */
export type Voucher = (call: Request) => Promise<Authentication | undefined>;

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
    /** The routes of each protocol the package implements, by the protocol's package. */
    readonly protocols?: ReadonlyMap<string, ReadonlySet<string>>;
    /** Wait until the package can answer before it serves. */
    readonly ready?: (signal: AbortSignal, health?: Health) => Promise<void>;
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
