import type * as sync from "@destack/sync";
import { Failure } from "@destack/error";
import { Subject, SyncError } from "@destack/sync";
import { Bearer } from "@destack/access/token";
import { type Identifier, type JsonObject, type JsonValue, present, schema } from "@destack/schema";
import {
    BearerProtocol,
    type ClientSocket,
    type Environment,
    WebSocketClosure,
    WebSocketUpgrade,
} from "@destack/environment";
import {
    createORPCClient,
    ErrorEvent,
    isORPCErrorStatus,
    mapEventIterator,
    ORPCError,
} from "@orpc/client";
import { isAsyncIteratorObject } from "@orpc/shared";
import {
    StandardRPCJsonSerializer,
    StandardRPCLink,
    StandardRPCSerializer,
    type StandardLinkClient,
} from "@orpc/client/standard";
import { ClientPeer } from "@orpc/standard-server-peer";
import { CallError } from "../error/call.ts";
import { ObjectError } from "../error/error.ts";
import { MAX_PAGE_LIMIT } from "../method/page.ts";
import { Method } from "../method/method.ts";
import { ObjectDescription } from "../object/description.ts";
import type { ObjectType } from "../object/object.ts";
import type { Client } from "../procedure/index.ts";
import { traceCalls } from "../procedure/telemetry.ts";
import { procedureHeaders } from "../client/package.ts";
import { BOOKMARK_HEADER, Bookmark } from "../procedure/bookmark.ts";
import { ObjectRemote, type Push, type Signer } from "./remote.ts";
import {
    type ObjectQuery,
    type PushResult,
    storeProcedures,
    type StoreProcedures,
} from "./replica.ts";
import { ObjectFileClient } from "../file/client.ts";

/** The path of a store's connection, below the package serving the store. */
export const CONNECTION_PATH = "/store/connect";

/** The WebSocket subprotocol of a store's connection. */
export const CONNECTION_PROTOCOL = "destack.store";

/** The text frame a client probes its socket's liveness with, which the server's host answers. */
export const CONNECTION_PING = "ping";

/** The text frame the server's host answers a liveness probe with. */
export const CONNECTION_PONG = "pong";

/** How often a client probes its socket in milliseconds, a dead one showing within two probes. */
const HEARTBEAT_MILLISECONDS = 10_000;

/** The query parameter confining a connection to one scope, which a forwarding gateway sets. */
export const CONNECTION_SCOPE_PARAMETER = "scope";

/** The state of an open WebSocket (WHATWG WebSockets 4). */
const OPEN = 1;

/** A page of a list method: its rows and the cursor continuing it, null after the last. */
const ListPage = schema.looseObject({
    items: schema.array(schema.jsonObject()),
    cursor: schema.string().nullable(),
});

/** The store procedures a connection calls, as one router. */
const STORE_ROUTER = { store: storeProcedures };

/** One socket a connection opened, with the peer numbering its calls. */
interface Opened {
    /** The URL the socket opened at. */
    readonly url: string;
    /** The socket. */
    readonly socket: ClientSocket;
    /** The peer encoding the calls and decoding their answers and pages. */
    readonly peer: ClientPeer;
}

/** Where a connection opens its sockets: at a URL, or where it is located at each opening. */
export type ConnectionTarget =
    | {
          /** The URL of the store's connection, `ws:` or `wss:`. */
          readonly url: string;
      }
    | {
          /** Read where the store's connection is at each opening. */
          readonly locate: () => Promise<string>;
      };

/** Where and how a connection opens its sockets. */
export type ConnectionOptions = ConnectionTarget & ConnectionSettings;

/** How a connection opens its sockets and calls through them. */
export interface ConnectionSettings {
    /** The scope the follower replicates, absent for a connection to a whole home. */
    readonly scope?: string;
    /** Add the caller's credential to each opening and each call. */
    readonly authorize?: (request: Request) => Promise<Request>;
    /** Resolve a refusal, such as by stepping up, answering whether to call again. */
    readonly consent?: (refused: unknown) => Promise<boolean>;
    /** The watermarks each call requires and observes, shared with the caller's other clients. */
    readonly bookmark?: Bookmark;
    /** Open a WebSocket, the environment's network by default. */
    readonly websocket?: (url: string, protocols: readonly string[]) => ClientSocket;
    /** Read the connection URL of the machine serving a moved scope, absent to follow no moves. */
    readonly machine?: (machine: Identifier<"machine">) => string;
    /** Open the connection of an enclosing scope's home whose rows the follower changes. */
    readonly homes?: (scope: string) => ObjectConnection;
    /** Open a connection as a signing principal, absent for a follower that signs as none. */
    readonly signing?: (signer: Signer) => ObjectConnection;
    /** The environment: the network opens the sockets, the clock and randomness name calls. */
    readonly environment: Environment;
}

/** A scope's calls and follows over a store's socket, which the origin's scopes share. */
export class ObjectConnection implements ObjectRemote {
    /** The scope the follower replicates, absent for a connection to a whole home. */
    readonly scope: string | undefined;
    /** Where and how the connection opens its socket. */
    readonly #options: ConnectionOptions;
    /** The socket, shared with the connections of the origin's other scopes. */
    readonly #shared: StoreSocket;
    /** The socket to the machine serving the scope once it moved, which this connection owns. */
    #moved: StoreSocket | undefined;
    /** The connections of enclosing scopes' homes, by scope. */
    readonly #homes = new Map<string, ObjectConnection>();
    /** The connections signing as a principal, by its key. */
    readonly #signers = new Map<string, ObjectConnection>();

    /** Connect a scope over a socket, requiring the scope for a connection with enclosing homes. */
    private constructor(
        scope: string | undefined,
        options: ConnectionOptions,
        socket: StoreSocket,
    ) {
        // take the scope, its options and the shared socket, refusing enclosing homes without a scope
        this.scope = scope;
        this.#options = options;
        this.#shared = socket;
        if (options.homes !== undefined) {
            present(scope, "the scope the enclosing homes enclose");
        }
    }

    /** Connect to the store at a URL, opening the socket at the first call. */
    static open(options: ConnectionOptions): ObjectConnection {
        const locate = "locate" in options ? options.locate : async () => options.url;

        return new ObjectConnection(options.scope, options, new StoreSocket(options, locate));
    }

    /** Connect another scope over the same socket. */
    at(scope: string): ObjectConnection {
        return new ObjectConnection(scope, this.#options, this.#shared);
    }

    /** The store procedures over the socket. */
    get store(): Client<StoreProcedures> {
        return this.#socket.store;
    }

    /** The socket serving the scope: its own after a move, else the origin's shared one. */
    get #socket(): StoreSocket {
        return this.#moved ?? this.#shared;
    }

    /** The URL the connection opens at, absent for one located at each opening. */
    get url(): string | undefined {
        return "url" in this.#options ? this.#options.url : undefined;
    }

    /** Open the client of the store's files, which sends as the caller. */
    async files(): Promise<ObjectFileClient> {
        // find where the store is mounted
        const { authorize, environment } = this.#options;
        const url = new URL((await this.#socket.locate()).replace(/^ws/u, "http"));
        url.pathname = url.pathname.slice(0, -CONNECTION_PATH.length);

        return new ObjectFileClient({
            url: url.href,
            environment,
            fetch: async (request) =>
                environment.network.fetch(
                    authorize === undefined ? request : await authorize(request),
                ),
        });
    }

    /** Open connections to stores mounted at URLs under bearer secrets, one per URL and secret. */
    static pool(environment: Environment): (url: string, secret: string) => ObjectConnection {
        const connections = new Map<string, ObjectConnection>();

        return (url, secret) => {
            // reuse the connection of a URL and secret
            const key = JSON.stringify([url, secret]);
            const known = connections.get(key);
            if (known !== undefined) {
                return known;
            }

            // connect to a new one, signing its upgrade and each call with the secret
            const connection = ObjectConnection.open({
                url: ObjectConnection.url(url),
                authorize: async (request) => {
                    const headers = new Headers(request.headers);
                    headers.set("authorization", `Bearer ${secret}`);

                    return new Request(request, { headers });
                },
                environment,
            });
            connections.set(key, connection);

            return connection;
        };
    }

    /** Read the connection URL of the store of a package mounted at a URL. */
    static url(mounted: string): string {
        return `${mounted.replace(/^http/u, "ws")}${CONNECTION_PATH}`;
    }

    /** Report whether a request upgrades to a store's connection. */
    static isUpgrade(request: Request): boolean {
        return (
            WebSocketUpgrade.of(request) !== undefined &&
            new URL(request.url).pathname === CONNECTION_PATH
        );
    }

    /** Stream a replica's pages from the store serving the scope. */
    async *stream(subscription: sync.Subscription, signal: AbortSignal): AsyncGenerator<sync.Page> {
        yield* await this.#call((store) => store.stream(subscription, { signal }));
    }

    /** Record a subscription's moved parameters at the store streaming it. */
    async request(subscription: sync.Subscription): Promise<void> {
        await this.#call((store) => store.request({ subscription }));
    }

    /** Push one home's mutations, reading the replica's watermark after a home commits. */
    async push(home: string, push: Push, signal?: AbortSignal): Promise<PushResult> {
        // push signed mutations as their signer
        const { mutations, writerId, chain, signer, origin } = push;
        const options = signal === undefined ? {} : { signal };
        const input = {
            scope: home,
            mutations: [...mutations],
            ...(writerId === undefined ? {} : { writerId }),
            ...(origin === undefined ? {} : { origin }),
        };
        if (signer !== undefined) {
            return this.#signed(signer).store.push(input, options);
        }

        // push to the store serving the scope, which forwards replicated rows to their home
        if (home === this.scope || this.#options.homes === undefined) {
            return this.#call((store) => store.push(input, options));
        }

        // push an enclosing scope's mutations to its home
        const pushed = await this.home(home).store.push(input, options);

        // read the replica's watermark, waiting for the home's commit when the chain has the home
        const homes = chain.includes(home) ? [pushed.watermark] : [];
        const scope = present(this.scope, "the enclosed scope");
        const watermark = await this.#call((store) => store.watermark({ scope, homes }, options));

        return { outcomes: pushed.outcomes, queued: pushed.queued, watermark };
    }

    /** Answer a reading call at the scope's store, as a signer or the caller. */
    async call(scope: string, call: sync.Call, signer?: Signer): Promise<JsonValue> {
        return signer === undefined
            ? this.#call((store) => store.call({ scope, call }))
            : this.#signed(signer).store.call({ scope, call });
    }

    /** Send an unstored event to an object's readers, as a signing principal or the caller. */
    async broadcast(
        scope: string,
        object: Omit<sync.ObjectReference, "scope">,
        event: JsonValue,
        signer?: Signer,
    ): Promise<void> {
        // send as the signer on its own connection
        if (signer !== undefined) {
            await this.#signed(signer).store.broadcast({ scope, object, event });

            return;
        }

        // send as the caller
        await this.#call((store) => store.broadcast({ scope, object, event }));
    }

    /** Describe object types the store knows, every served one when given none. */
    async describe(types?: readonly sync.ObjectTypeReference[]): Promise<ObjectDescription[]> {
        // ask nothing for an empty list
        if (types?.length === 0) {
            return [];
        }

        // describe them at the store serving the scope
        const { objects } = await this.#call((store) =>
            store.describe(types === undefined ? {} : { types: [...types] }),
        );

        return objects.map((object) => ObjectDescription.parse(object));
    }

    /** Run a routed method by name with JSON input in the follower's scope. */
    async run(object: ObjectType, name: string, input: JsonObject): Promise<JsonValue> {
        const scope = present(this.scope, `the scope '${object.name}.${name}' acts in`);

        return ObjectRemote.run(this, object, name, input, scope, this.#options.environment);
    }

    /** List a type's rows through its list method, page by page up to the query's limit. */
    async list(
        object: ObjectType,
        query: Pick<ObjectQuery, "where" | "orderBy" | "limit" | "permission"> &
            Readonly<Record<string, unknown>> = {},
    ): Promise<readonly JsonObject[]> {
        // find the type's list method
        const method = Object.entries(object.methods).find(
            ([, declared]) => declared.kind === "list" && Method.isRouted(declared),
        )?.[0];
        if (method === undefined) {
            throw ObjectError.of(
                "LIST_METHOD_NOT_FOUND",
                `list method of ${object.name} not found`,
            );
        }

        // read pages until the last one or the limit, each continuing from the one before
        const input = schema.jsonObject().parse(query);
        const { limit } = query;
        const rows: JsonObject[] = [];
        let cursor: string | null = null;
        do {
            const size = Math.min(MAX_PAGE_LIMIT, (limit ?? Infinity) - rows.length);
            const page = ListPage.parse(
                await this.run(object, method, {
                    ...input,
                    limit: size,
                    ...(cursor === null ? {} : { cursor }),
                }),
            );
            rows.push(...page.items);
            cursor = page.cursor;
        } while (cursor !== null && rows.length < (limit ?? Infinity));

        return rows;
    }

    /** Connect to a scope's home: this connection for its scope, else each enclosing home once. */
    home(home: string): ObjectConnection {
        // call the follower's scope here, refusing another scope without homes
        const { homes } = this.#options;
        if (home === this.scope) {
            return this;
        } else if (homes === undefined) {
            throw ObjectError.of(
                "HOME_INVALID",
                `home '${home}' is invalid: the follower of '${String(this.scope)}' cannot reach it`,
            );
        }

        // connect to each other scope's home once
        const connected = this.#homes.get(home) ?? homes(home);
        this.#homes.set(home, connected);

        return connected;
    }

    /** Close the sockets and the homes' and signers' connections, failing their calls as lost. */
    close(): void {
        this.#shared.close();
        this.#moved?.close();
        for (const connection of [...this.#homes.values(), ...this.#signers.values()]) {
            connection.close();
        }
    }

    /** Call the store serving the scope, at its machine's own socket once the scope moved. */
    async #call<Value>(send: (store: Client<StoreProcedures>) => Promise<Value>): Promise<Value> {
        try {
            return await this.#socket.call(send);
        } catch (error) {
            // rethrow anything but a move to a machine this connection can reach
            const moved = CallError.data(error, "MISDIRECTED_REQUEST");
            const { machine } = this.#options;
            if (moved === undefined || !("machine" in moved) || machine === undefined) {
                throw error;
            }

            // open a socket of the scope's own at the machine, leaving the origin's to its other scopes
            const url = machine(moved.machine);
            this.#moved?.close();
            this.#moved = new StoreSocket(this.#options, async () => url);

            return this.#moved.call(send);
        }
    }

    /** Connect as a signing principal once, refusing a follower that signs as none. */
    #signed(signer: Signer): ObjectConnection {
        // reuse the signer's connection, else open one, by its subject, actors and grants
        const signing = present(this.#options.signing, "the connection's signing connections");
        const key = JSON.stringify([
            Subject.key(signer.subject),
            signer.actors.map((actor) => Subject.key(actor)),
            signer.grants.map((grant) => grant.id),
        ]);
        const connected = this.#signers.get(key) ?? signing(signer);
        this.#signers.set(key, connected);

        return connected;
    }
}

/** One socket to a store, opened at the first call and again at the next after it drops. */
class StoreSocket {
    /** The store procedures over the socket. */
    readonly store: Client<StoreProcedures>;
    /** How the socket opens. */
    readonly #options: ConnectionOptions;
    /** Read where the socket opens at each opening. */
    readonly #locate: () => Promise<string>;
    /** The socket opening or open now, absent before the first call and after a close. */
    #opened: Promise<Opened> | undefined;
    /** Why each closed socket closed, which its follows fail with whenever they read next. */
    readonly #closes = new WeakMap<ClientSocket, Error>();
    /** The socket the opening above opened, which alone clears it as it closes. */
    #live: ClientSocket | undefined;

    /** Prepare a socket opening where located, calling and tracing the procedures through it. */
    constructor(options: ConnectionOptions, locate: () => Promise<string>) {
        // open where located, calling through the link and tracing each call
        this.#options = options;
        this.#locate = locate;
        const link = this.#link();
        const traced = traceCalls("client", "orpc", options.environment);
        const client = createORPCClient<Client<typeof STORE_ROUTER>>({
            call: (path, input, call) => traced(path, () => link.call(path, input, call)),
        });
        this.store = client.store;
    }

    /** Close the socket, failing its calls and follows as lost. */
    close(): void {
        this.#live?.close();
        this.#live = undefined;
        this.#opened = undefined;
    }

    /** Call the store, again once the caller resolved a refusal. */
    async call<Value>(send: (store: Client<StoreProcedures>) => Promise<Value>): Promise<Value> {
        try {
            return await send(this.store);
        } catch (error) {
            // call again once the caller resolved the refusal
            const { consent } = this.#options;
            if (consent !== undefined && (await consent(error))) {
                return send(this.store);
            }

            throw error;
        }
    }

    /** Read the socket open now, opening one once the last closed. */
    #opening(): Promise<Opened> {
        // take the socket opening already, else open one, forgetting an open that fails before its socket
        if (this.#opened === undefined) {
            const opening = this.#open();
            this.#opened = opening;
            opening.catch(() => {
                if (this.#opened === opening) {
                    this.#opened = undefined;
                }
            });
        }

        return this.#opened;
    }

    /** Open a socket with a fresh credential, settling once it opens. */
    async #open(): Promise<Opened> {
        // open where the store is now, offering its protocol and the caller's bearer token
        const { environment } = this.#options;
        const located = await this.#locate();
        const token = await this.#token(located).catch((error: unknown) => {
            // read an exchange that failed without a refusal, such as over a lost network, as a lost connection
            if (!Failure.isRefusal(error)) {
                throw SyncError.of(
                    "CONNECTION_UNAVAILABLE",
                    `connection is unavailable: the credential for '${located}' did not arrive: ${Failure.of(error).message}`,
                    { cause: error },
                );
            }
            throw error;
        });
        const protocols = [
            CONNECTION_PROTOCOL,
            ...(token === undefined ? [] : [BearerProtocol.write(token)]),
        ];
        const websocket =
            this.#options.websocket ??
            ((url: string, offered: readonly string[]) =>
                environment.network.websocket(url, offered));
        const socket = websocket(located, protocols);
        this.#live = socket;

        // number the calls, failing them as lost once the socket closes, and open anew at the next call
        const peer = new ClientPeer((message) => socket.send(StoreSocket.#text(message)));
        let isAnswered = true;
        let refused: CallError<string, unknown> | undefined;
        socket.addEventListener("message", (event) => {
            // read a refused socket's one frame as its refusal, take a liveness answer, and hand any other frame to the peer
            const frame = String(event.data);
            if (socket.protocol !== CONNECTION_PROTOCOL) {
                refused = StoreSocket.#refused(frame);
            } else if (frame === CONNECTION_PONG) {
                isAnswered = true;
            } else {
                void peer.message(frame);
            }
        });

        // probe the socket while it is open, closing it once a probe goes unanswered
        const closing = new AbortController();
        socket.addEventListener("close", () => closing.abort(), { once: true });
        socket.addEventListener(
            "open",
            () => {
                // probe only a socket that speaks the store's protocol
                if (socket.protocol !== CONNECTION_PROTOCOL) {
                    return;
                }
                void (async () => {
                    while (!closing.signal.aborted) {
                        await environment.clock.sleep(HEARTBEAT_MILLISECONDS, closing.signal);
                        if (!isAnswered) {
                            socket.close(WebSocketClosure.goingAway, "liveness probe unanswered");

                            return;
                        }
                        isAnswered = false;
                        socket.send(CONNECTION_PING);
                    }
                })().catch((error: unknown) => {
                    // end quietly on the close's own abort, failing loudly on anything else
                    if (!closing.signal.aborted) {
                        throw error;
                    }
                });
            },
            { once: true },
        );
        socket.addEventListener("close", (event) => {
            // fail the socket's calls and follows with its refusal or why it closed, and open anew at the next call
            const lost = refused ?? StoreSocket.#lost(located, event);
            this.#closes.set(socket, lost);
            peer.close({ reason: lost });
            if (this.#live === socket) {
                this.#live = undefined;
                this.#opened = undefined;
            }
        });

        // settle once the socket opens speaking the store's protocol, failing with its refusal or loss once it closes first
        return new Promise<Opened>((resolve, reject) => {
            socket.addEventListener(
                "open",
                () => {
                    if (socket.protocol === CONNECTION_PROTOCOL) {
                        resolve({ url: located, socket, peer });
                    }
                },
                { once: true },
            );
            socket.addEventListener(
                "close",
                (event) => reject(refused ?? StoreSocket.#lost(located, event)),
                { once: true },
            );
        });
    }

    /** Send calls through the open socket with the release and edition each procedure speaks. */
    #link(): StandardRPCLink<Record<never, never>> {
        // read failures with their reason and domain, which oRPC's own decoding drops
        const serializer = new StandardRPCSerializer(new StandardRPCJsonSerializer());
        const sender: StandardLinkClient<Record<never, never>> = {
            call: async (request) => {
                // send through the socket open now, refusing one that closes
                const { url, socket, peer } = await this.#opening();
                if (socket.readyState !== OPEN) {
                    throw SyncError.of(
                        "CONNECTION_UNAVAILABLE",
                        `connection is unavailable: the socket to '${url}' is closing`,
                    );
                }
                const response = await peer.request(request);

                return { ...response, body: () => Promise.resolve(response.body) };
            },
        };
        const headersOf = procedureHeaders(STORE_ROUTER);

        return new StandardRPCLink(sender, {
            url: "http://orpc",
            headers: async (_options, path) => {
                // send the release, the edition, the watermarks the call requires and the caller's token, as each HTTP call did
                const { bookmark } = this.#options;
                const isBookmarked = bookmark !== undefined && bookmark.watermarks.length > 0;
                const { url } = await this.#opening();
                const token = await this.#token(url);

                return {
                    ...headersOf(path),
                    ...(isBookmarked ? { [BOOKMARK_HEADER]: bookmark.format() } : {}),
                    ...(token === undefined ? {} : { authorization: `Bearer ${token}` }),
                };
            },
            clientInterceptors: [
                async (call) => {
                    // observe the answer's watermarks
                    const response = await call.next();
                    const observed = response.headers[BOOKMARK_HEADER];
                    const { bookmark } = this.#options;
                    if (bookmark !== undefined && typeof observed === "string") {
                        for (const watermark of Bookmark.parse(observed).watermarks) {
                            bookmark.observe(watermark);
                        }
                    }

                    // pass on an answer
                    if (!isORPCErrorStatus(response.status)) {
                        return response;
                    }

                    // read a refusal as its call error
                    const body: unknown = await response.body();
                    const refused = CallError.fromJson(serializer.deserialize(body));
                    throw refused ?? new ORPCError("INTERNAL_SERVER_ERROR", { data: body });
                },
            ],
            interceptors: [
                async (call) => {
                    // read a stream's failure as its call error, or as its own abort or its socket's close
                    const output: unknown = await call.next();
                    const socket = this.#live;
                    const { signal } = call;
                    const ended = (): unknown => {
                        // end with the stream's own abort, else with its socket's close
                        if (signal?.aborted === true) {
                            const reason: unknown = signal.reason;

                            return reason;
                        }

                        return socket === undefined ? undefined : this.#closes.get(socket);
                    };

                    return StoreSocket.#readStreamed(output, serializer, ended);
                },
            ],
        });
    }

    /** Read where the socket opens. */
    locate(): Promise<string> {
        return this.#locate();
    }

    /** Read the caller's bearer token for a call at a URL, none for a caller without one. */
    async #token(url: string): Promise<string | undefined> {
        const { authorize } = this.#options;
        const authorized =
            authorize === undefined
                ? undefined
                : await authorize(new Request(url.replace(/^ws/u, "http")));

        return Bearer.token(authorized?.headers.get("authorization") ?? null);
    }

    /** Read a stream's failures with their reason and domain, or its abort or socket's close. */
    static #readStreamed(
        output: unknown,
        serializer: StandardRPCSerializer,
        ended: () => unknown,
    ): unknown {
        return isAsyncIteratorObject(output)
            ? mapEventIterator(output, {
                  value: async (value: unknown) => value,
                  error: async (error: unknown) => {
                      // read the failure from the event it arrived in, as oRPC read it or left it
                      const cause = error instanceof ORPCError ? error.cause : undefined;
                      const refused =
                          error instanceof ErrorEvent
                              ? CallError.fromJson(error.data)
                              : cause instanceof ErrorEvent
                                ? CallError.fromJson(serializer.deserialize(cause.data))
                                : undefined;

                      return ended() ?? refused ?? error;
                  },
              })
            : output;
    }

    /** Send a peer's message as a text frame. */
    static #text(message: string | ArrayBufferLike | Uint8Array): string {
        return typeof message === "string"
            ? message
            : new TextDecoder().decode(
                  message instanceof Uint8Array ? message : new Uint8Array(message),
              );
    }

    /** Read the refusal a refused socket states in its one frame. */
    static #refused(frame: string): CallError<string, unknown> {
        const refused = CallError.fromJson(JSON.parse(frame));
        if (refused === undefined) {
            throw new TypeError("a refused socket states its refusal as a call error's body");
        }

        return refused;
    }

    /** Fail a call or follow whose socket closed, for its caller to resume. */
    static #lost(url: string, closed: CloseEvent): Error {
        const { code, reason } = closed;
        const message =
            reason === ""
                ? `connection is unavailable: the socket to '${url}' closed with '${code}'`
                : `connection is unavailable: the socket to '${url}' closed with '${code}': '${reason}'`;

        return SyncError.of("CONNECTION_UNAVAILABLE", message);
    }
}
