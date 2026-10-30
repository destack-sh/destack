import { BunNetwork } from "./bun.ts";
import { BunServer } from "../server/bun.ts";
import { Network } from "./network.ts";
import type {
    Listener,
    Server,
    ServeOptions,
    ServerSocket,
    ServerSocketHandler,
} from "../server/server.ts";
import { NetworkError } from "./error.ts";
import {
    type ConnectOptions,
    HostAddress,
    type SocketAddress,
    refuseUnixTls,
    socketName,
    SocketInbox,
    type SocketListener,
    type SocketListenOptions,
    type Socket,
} from "./socket/socket.ts";
import { Random } from "../random/random.ts";
import type { SimulatedClock } from "../clock/simulated.ts";
import type { SimulationEvent } from "../simulation/simulation.ts";
import { ABNORMAL_CLOSURE, NORMAL_CLOSURE, SimulatedWebSocket } from "./websocket.ts";

/** The prefix of a Unix socket's key, which every node sharing the host's files reaches. */
const UNIX_KEY = "unix:";

/** The first dynamic port, where simulated listeners on port zero start (RFC 6335). */
const FIRST_DYNAMIC_PORT = 49152;

/** The default ports of the schemes requests and sockets use (RFC 9110, RFC 6455). */
const DEFAULT_PORTS: Readonly<Record<string, string>> = {
    "http:": "80",
    "https:": "443",
    "ws:": "80",
    "wss:": "443",
};

/** The close code of a server going away (RFC 6455). */
const GOING_AWAY = 1001;

/** The faults a simulated network applies to every delivery beside its partitions. */
export interface NetworkFaults {
    /** The share of deliveries lost, from 0 to 1; a lost message breaks its connection. */
    readonly drop?: number;
    /** The latency of a delivery, from its least to its most, in milliseconds. */
    readonly delay?: { readonly least: number; readonly most: number };
}

/** A direction of a connection. */
type Direction = "toServer" | "toClient";

/** A listener's answer to a request: a response, or an accepted socket. */
type Answer =
    | { readonly kind: "response"; readonly response: Response }
    | { readonly kind: "socket"; readonly end: SocketEnd; readonly protocol: string };

/** The server's end of a simulated connection, through its listener's handler. */
interface SocketEnd {
    /** Take the open, with the verbs that send and close toward the client. */
    readonly open: (peer: {
        readonly send: (message: string | Uint8Array<ArrayBuffer>) => void;
        readonly close: (code: number, reason: string) => void;
    }) => void;
    /** Take a message. */
    readonly message: (message: string | Uint8Array<ArrayBuffer>) => void;
    /** Take the close. */
    readonly close: (code: number, reason: string) => void;
}

/** A listener's place on the network. */
interface Endpoint {
    /** The node serving it. */
    readonly node: string;
    /** Answer a request. */
    readonly answer: (request: Request) => Promise<Answer>;
}

/** An open connection between a client's socket and a listener's handler. */
interface Connection {
    /** The client's node. */
    readonly client: string;
    /** The server's node. */
    readonly server: string;
    /** The listener's key: its host name and port, or its Unix socket's path. */
    readonly address: string;
    /** The client's socket. */
    readonly socket: SimulatedWebSocket;
    /** The server's end. */
    readonly end: SocketEnd;
    /** When the last delivery in each direction arrives, which keeps each direction in order. */
    readonly arrivals: Record<Direction, number>;
    /** Whether the connection still carries messages. */
    isOpen: boolean;
}

/** A socket listener's place on the network. */
interface SocketEndpoint {
    /** The node listening. */
    readonly node: string;
    /** Take an accepted connection's server end. */
    readonly accept: (socket: Socket) => void;
}

/** One way of a socket connection: the receiving end's inbox and when its last delivery arrives. */
interface SocketWay {
    /** The receiving end's inbox, which a TLS upgrade replaces. */
    inbox: SocketInbox;
    /** When the last delivery arrives, which keeps the bytes in order. */
    arrival: number;
    /** Whether the way carries bytes, ends after its bytes in flight, or ended at the receiver. */
    state: "open" | "ending" | "ended";
}

/** An open socket connection between a client node and a listening node. */
interface SocketConnection {
    /** The client's node. */
    readonly client: string;
    /** The server's node. */
    readonly server: string;
    /** The listener's key: its host name and port, or its Unix socket's path. */
    readonly address: string;
    /** The ways toward the server and toward the client. */
    readonly ways: Record<Direction, SocketWay>;
    /** The settlers of its ends' closes. */
    readonly closes: Set<() => void>;
    /** Whether the connection still carries bytes. */
    isOpen: boolean;
    /** The failure that broke it, absent while open or once closed. */
    broken?: TypeError;
}

/** A listener of a simulated node, answering through its handlers. */
class SimulatedListener<Attachment> implements Listener<Attachment> {
    /** The URL it listens at. */
    readonly url: URL;
    /** The handlers. */
    readonly #options: ServeOptions<Attachment>;
    /** The upgrades accepted while their request is answered. */
    readonly #upgrades = new WeakMap<
        Request,
        { readonly attachment: Attachment; readonly protocol: string }
    >();
    /** Stop listening. */
    readonly #stop: (isClosingActive: boolean) => void;

    /** Listen at a URL with handlers. */
    constructor(
        url: URL,
        options: ServeOptions<Attachment>,
        stop: (isClosingActive: boolean) => void,
    ) {
        this.url = url;
        this.#options = options;
        this.#stop = stop;
    }

    /** Accept a request's upgrade with an attachment. */
    upgrade(
        request: Request,
        options: { readonly attachment: Attachment; readonly headers?: Record<string, string> },
    ): boolean {
        // refuse a request that asks for no WebSocket
        if (request.headers.get("upgrade") !== "websocket") {
            return false;
        }

        // store the attachment and the chosen subprotocol until the answer
        const protocol = options.headers?.["sec-websocket-protocol"] ?? "";
        this.#upgrades.set(request, { attachment: options.attachment, protocol });

        return true;
    }

    /** Stop listening. */
    stop(isClosingActive = false): Promise<void> {
        this.#stop(isClosingActive);

        return Promise.resolve();
    }

    /** Answer a request through the handler, as a response or an accepted socket. */
    async answer(request: Request): Promise<Answer> {
        // answer with a response
        const response = await this.#options.fetch(request, this);
        const upgrade = this.#upgrades.get(request);
        if (response !== undefined) {
            return { kind: "response", response };
        } else if (upgrade === undefined) {
            throw new TypeError(`${this.url.origin} sent no response and no upgrade`);
        }

        // accept the socket with its attachment
        return {
            kind: "socket",
            end: this.#end(upgrade.attachment, this.#options.websocket),
            protocol: upgrade.protocol,
        };
    }

    /** Make the server's end of an accepted socket. */
    #end(attachment: Attachment, handler: ServerSocketHandler<Attachment> | undefined): SocketEnd {
        // refuse sockets without a handler
        if (handler === undefined) {
            throw new TypeError(`${this.url.origin} accepted a socket without a websocket handler`);
        }

        // hand the handler the server's socket once the connection opens
        let socket: ServerSocket<Attachment> | undefined;

        return {
            open: (peer) => {
                socket = {
                    attachment,
                    send: peer.send,
                    close: (code = NORMAL_CLOSURE, reason = "") => peer.close(code, reason),
                };
                handler.open?.(socket);
            },
            message: (message) => {
                if (socket !== undefined) {
                    handler.message(socket, message);
                }
            },
            close: (code, reason) => {
                if (socket !== undefined) {
                    handler.close?.(socket, code, reason);
                }
            },
        };
    }
}

/** The network between a simulation's nodes: listeners, sockets, latency, loss and partitions. */
export class SimulatedNetwork {
    /** The listeners by host name and port, which TLS and the scheme leave alike. */
    readonly #endpoints = new Map<string, Endpoint>();
    /** The open connections. */
    readonly #connections = new Set<Connection>();
    /** The socket listeners by key. */
    readonly #socketEndpoints = new Map<string, SocketEndpoint>();
    /** The open socket connections. */
    readonly #socketConnections = new Set<SocketConnection>();
    /** The pairs of nodes that cannot reach each other, as `a b` keys. */
    readonly #partitions = new Set<string>();
    /** The nodes cut off from every other, such as browsers gone offline. */
    readonly #isolated = new Set<string>();
    /** The listeners of nodes leaving or joining the network. */
    readonly #isolationListeners = new Set<(node: string, isIsolated: boolean) => void>();
    /** The deliveries begun so far. */
    #sent = 0;
    /** The deliveries settled so far. */
    #delivered = 0;
    /** The next free port. */
    #port = FIRST_DYNAMIC_PORT;
    /** The faults every delivery meets. */
    #faults: NetworkFaults = {};
    /** The simulation's clock, which latency waits on. */
    readonly #clock: SimulatedClock;
    /** The simulation's randomness, which latency and loss draw from. */
    readonly #random: Random;
    /** Record what happens, in order. */
    readonly #record: (event: SimulationEvent) => void;

    /** Connect the network to its simulation's clock, randomness and trace. */
    constructor(clock: SimulatedClock, random: Random, record: (event: SimulationEvent) => void) {
        this.#clock = clock;
        this.#random = random;
        this.#record = record;
    }

    /** The deliveries begun so far, which settle on simulated time without holding it. */
    get sent(): number {
        return this.#sent;
    }

    /** The deliveries settled so far. */
    get delivered(): number {
        return this.#delivered;
    }

    /** Read the network a node reaches other origins through, without TLS. */
    of(node: string): Network {
        return {
            fetch: async (request, options) => {
                // send through a Unix socket on the host to a host process, which may call back in
                if (options?.unix !== undefined) {
                    return this.#clock.trackPeer(BunNetwork.fetch(request, options));
                }

                // refuse a private host by its name, the simulated network routing by name
                if (options?.isPublicOnly === true) {
                    await Network.requirePublic(this.of(node), new URL(request.url).hostname);
                }

                // decode the response's content encoding, as Bun's fetch does unless asked to keep it
                const response = await this.#transit(this.#fetch(node, request));

                return options?.decompress === false ? response : decoded(response);
            },
            connect: (address, options) => {
                // refuse a private host by its name, the simulated network routing by name
                const isPrivate = !("unix" in address) && !HostAddress.isPublic(address.hostname);
                if (options?.isPublicOnly === true && isPrivate) {
                    throw NetworkError.of(
                        "ADDRESS_PRIVATE",
                        `${address.hostname} resolves to ${address.hostname}, outside the public network`,
                    );
                }

                return this.#connect(node, address, options);
            },
            websocket: (url, protocols) => this.#websocket(node, url, protocols),
            // route by name, each name its own address
            resolve: async (hostname) => [hostname],
        };
    }

    /** Read the server a node accepts requests, WebSockets and stream sockets through. */
    server(node: string): Server {
        return {
            http: (options) => this.#serve(node, options),
            socket: async (options) => this.#listen(node, options),
        };
    }

    /** Publish a node's listener on the host at its port, outside the simulated links. */
    publish<Attachment>(node: string, options: ServeOptions<Attachment>): Listener<Attachment> {
        // listen on the host at a free port unless given, counting its answers as the world's work
        const host = BunServer.http({
            ...options,
            fetch: (request, listener) =>
                this.#clock.track(Promise.resolve(options.fetch(request, listener))),
        });

        // serve the node at the same address, stopping both together
        const simulated = this.#serve(node, { ...options, port: Number(host.url.port) });

        return {
            url: simulated.url,
            upgrade: simulated.upgrade,
            stop: async (isClosingActive) => {
                await Promise.all([host.stop(isClosingActive), simulated.stop(isClosingActive)]);
            },
        };
    }

    /** Cut the network between two nodes, both ways, breaking their connections. */
    partition(left: string, right: string): void {
        // cut the pair
        const pair = SimulatedNetwork.#pair(left, right);
        this.#partitions.add(pair);
        this.#record({ kind: "partition", at: this.#clock.now(), nodes: [left, right] });

        // break its connections
        for (const connection of this.#connections) {
            if (SimulatedNetwork.#pair(connection.client, connection.server) === pair) {
                this.#break(connection);
            }
        }
        for (const connection of this.#socketConnections) {
            if (SimulatedNetwork.#pair(connection.client, connection.server) === pair) {
                this.#breakSocket(connection);
            }
        }
    }

    /** Cut a node off from every other, breaking its connections, as a device going offline. */
    isolate(node: string): void {
        // cut the node off, telling the listeners of a change
        if (this.#isolated.has(node)) {
            return;
        }
        this.#isolated.add(node);
        this.#tellIsolation(node, true);
        this.#record({ kind: "partition", at: this.#clock.now(), nodes: [node] });

        // break the connections the isolation cuts
        for (const connection of this.#connections) {
            if (this.#isCut(connection.client, connection.server)) {
                this.#break(connection);
            }
        }
        for (const connection of this.#socketConnections) {
            if (this.#isSocketCut(connection.address, connection.client, connection.server)) {
                this.#breakSocket(connection);
            }
        }
    }

    /** Join an isolated node to the network again. */
    rejoin(node: string): void {
        // join the node, telling the listeners of a change
        if (!this.#isolated.delete(node)) {
            return;
        }
        this.#record({ kind: "heal", at: this.#clock.now(), nodes: [node] });
        this.#tellIsolation(node, false);
    }

    /** Report whether a node is cut off from every other. */
    isIsolated(node: string): boolean {
        return this.#isolated.has(node);
    }

    /** Call a listener as each node leaves or joins the network, until disposed. */
    onIsolation(listener: (node: string, isIsolated: boolean) => void): Disposable {
        this.#isolationListeners.add(listener);

        return { [Symbol.dispose]: () => this.#isolationListeners.delete(listener) };
    }

    /** Tell the listeners a node left or joined the network. */
    #tellIsolation(node: string, isIsolated: boolean): void {
        for (const listener of this.#isolationListeners) {
            listener(node, isIsolated);
        }
    }

    /** Report whether the network between two nodes is cut, never a node from itself. */
    #isCut(left: string, right: string): boolean {
        return (
            left !== right &&
            (this.#isolated.has(left) ||
                this.#isolated.has(right) ||
                this.#partitions.has(SimulatedNetwork.#pair(left, right)))
        );
    }

    /** Join two nodes again, or every pair without nodes. */
    heal(left?: string, right?: string): void {
        // join the pair, or every pair
        if (left === undefined || right === undefined) {
            this.#partitions.clear();
        } else {
            this.#partitions.delete(SimulatedNetwork.#pair(left, right));
        }

        // record the healing
        const nodes = left === undefined || right === undefined ? [] : [left, right];
        this.#record({ kind: "heal", at: this.#clock.now(), nodes });
    }

    /** Set the faults every delivery meets. */
    degrade(faults: NetworkFaults): void {
        this.#faults = faults;
    }

    /** Drop a node's listeners and break its connections, as its process ends. */
    release(node: string): void {
        // stop the node's listeners
        for (const [origin, endpoint] of this.#endpoints) {
            if (endpoint.node === node) {
                this.#endpoints.delete(origin);
            }
        }
        for (const [address, endpoint] of this.#socketEndpoints) {
            if (endpoint.node === node) {
                this.#socketEndpoints.delete(address);
            }
        }
        for (const connection of this.#socketConnections) {
            if (connection.client === node || connection.server === node) {
                this.#breakSocket(connection);
            }
        }

        // break the node's connections
        for (const connection of this.#connections) {
            if (connection.client === node || connection.server === node) {
                this.#break(connection);
            }
        }
    }

    /** Listen as a node at a host name and port. */
    #serve<Attachment>(node: string, options: ServeOptions<Attachment>): Listener<Attachment> {
        // refuse Unix sockets, and an origin another listener serves
        if (options.unix !== undefined) {
            throw new TypeError("a simulated listener needs a host name and port");
        }
        const isFree = options.port === undefined || options.port === 0;
        const port = isFree ? this.#port : options.port;
        this.#port += isFree ? 1 : 0;
        const url = new URL(`http://${options.hostname ?? "localhost"}:${port}/`);
        const address = SimulatedNetwork.#address(url);
        if (this.#endpoints.has(address)) {
            throw new TypeError(`${address} is already served`);
        }

        // answer as the node until the listener stops, closing its connections when asked
        const listener: SimulatedListener<Attachment> = new SimulatedListener(
            url,
            options,
            (isClosingActive) => {
                // remove this listener alone, never one a restarted node serves at the address now
                if (this.#endpoints.get(address) === endpoint) {
                    this.#endpoints.delete(address);
                }
                for (const connection of this.#connections) {
                    if (isClosingActive && connection.address === address) {
                        this.#close(connection, "toClient", GOING_AWAY, "");
                    }
                }
            },
        );
        const endpoint = { node, answer: (request: Request) => listener.answer(request) };
        this.#endpoints.set(address, endpoint);

        return listener;
    }

    /** Listen for socket connections as a node at a host name and port, or at a Unix socket. */
    #listen(node: string, options: SocketListenOptions): SocketListener {
        // take the Unix socket, the given port, or the next free one for zero or none
        const requested = options.port ?? 0;
        const port = requested === 0 ? this.#port : requested;
        const listened: SocketAddress =
            options.unix === undefined
                ? { hostname: options.hostname ?? "localhost", port }
                : { unix: options.unix };
        this.#port += options.unix === undefined && requested === 0 ? 1 : 0;

        // refuse an address another listener has
        const address = socketKey(listened);
        if (this.#socketEndpoints.has(address)) {
            throw new TypeError(`${address} is already listened at`);
        }
        const endpoint = { node, accept: options.accept };
        this.#socketEndpoints.set(address, endpoint);

        return {
            address: listened,
            stop: async () => {
                // stop listening, never removing one a restarted node listens with now
                if (this.#socketEndpoints.get(address) === endpoint) {
                    this.#socketEndpoints.delete(address);
                }
                for (const connection of this.#socketConnections) {
                    if (connection.address === address) {
                        this.#breakSocket(connection);
                    }
                }
            },
            // keep nothing running, as the simulation's clock decides when it ends
            unref: () => {},
        };
    }

    /** Open a node's socket connection to a listener, TLS being a plain upgrade here. */
    #connect(from: string, address: SocketAddress, options: ConnectOptions = {}): Socket {
        // refuse TLS toward a Unix socket, and make the client's end, sending once the connection opened
        refuseUnixTls(address, options);
        const key = socketKey(address);
        const connecting = Promise.withResolvers<SocketConnection>();
        const inbox = SocketInbox.open(() => undefined);
        const client = this.#socketEnd({
            connected: connecting.promise,
            way: "toServer",
            inbox,
            connect: options,
        });

        // reach the listener, accepting the connection at its end after the latency
        const opened = (async () => {
            // refuse an address nothing listens at, a node cut off from it, and a lost connection
            const endpoint = this.#socketEndpoints.get(key);
            const at = this.#clock.now();
            if (endpoint === undefined) {
                this.#record({ kind: "refusal", at, from, target: key, reason: "unserved" });
                throw refusal(
                    `connection refused: nothing listens at ${socketName(address)}`,
                    "ConnectionRefused",
                );
            } else if (this.#isSocketCut(key, from, endpoint.node)) {
                this.#record({ kind: "refusal", at, from, target: key, reason: "partitioned" });
                throw refusal(
                    `connection failed: ${from} cannot reach ${endpoint.node}`,
                    "FailedToOpenSocket",
                );
            } else if (this.#isSocketLost(key, from, endpoint.node)) {
                this.#record({ kind: "loss", at, from, target: key });
                throw refusal(
                    `connection failed: the connection to ${key} was lost`,
                    "ConnectionClosed",
                );
            }
            await this.#clock.sleep(this.#latency());

            // open both ends, handing the server's to the listener
            const serverInbox = SocketInbox.open(() => undefined);
            const connection: SocketConnection = {
                client: from,
                server: endpoint.node,
                address: key,
                ways: {
                    toServer: { inbox: serverInbox, arrival: 0, state: "open" },
                    toClient: { inbox, arrival: 0, state: "open" },
                },
                closes: new Set(),
                isOpen: true,
            };
            this.#socketConnections.add(connection);
            this.#record({ kind: "request", at: this.#clock.now(), from, target: key });
            endpoint.accept(
                this.#socketEnd({
                    connected: Promise.resolve(connection),
                    way: "toClient",
                    inbox: serverInbox,
                    connect: undefined,
                }),
            );
            connecting.resolve(connection);
        })();
        void this.#transit(opened.catch((error: unknown) => connecting.reject(error)));

        return client;
    }

    /** Make one end of a connection: its inbox's readable, and a writable sending one way. */
    #socketEnd(options: {
        readonly connected: Promise<SocketConnection>;
        readonly way: Direction;
        readonly inbox: SocketInbox;
        readonly connect: ConnectOptions | undefined;
    }): Socket {
        // settle the end's close with its connection's, or at once as its connection fails
        const { connected, way, inbox, connect } = options;
        const closing = Promise.withResolvers<void>();
        let connection: SocketConnection | undefined;
        let isReplaced = false;
        const opened = connected.then(
            (open) => {
                connection = open;
                open.closes.add(() => closing.resolve());
            },
            (error: unknown) => {
                inbox.controller.error(error);
                closing.resolve();
                throw error;
            },
        );
        opened.catch(() => undefined);

        // send each written chunk, and the end, refusing a closed, broken or replaced connection
        const send = async (arrive: (open: SocketConnection) => void) => {
            const open = await connected;
            if (isReplaced) {
                throw new TypeError("the connection started TLS on another socket");
            } else if (!open.isOpen || open.ways[way].state !== "open") {
                throw open.broken ?? refusal("the connection closed", "ConnectionClosed");
            }
            this.#deliverSocket(open, way, () => arrive(open));
        };
        const writable = new WritableStream<Uint8Array>({
            write: async (chunk) => {
                const copied = chunk.slice();
                await send((open) => open.ways[way].inbox.controller.enqueue(copied));
            },
            close: async () => {
                this.#endSocket(await connected, way);
            },
        });
        const socket: Socket = {
            readable: inbox.readable,
            writable,
            opened,
            closed: closing.promise,
            // keep nothing running, as the simulation's clock decides when it ends
            unref: () => {},
            close: async () => {
                // end this way after its bytes in flight, once, and wait for the peer to end its own
                this.#endSocket(await connected, way);
                await closing.promise;
            },
            startTls: () => {
                // refuse an upgrade of a connection not connected for one, or not open yet
                if (connect?.secureTransport !== "starttls") {
                    throw new TypeError("only a connection connected with starttls can start TLS");
                } else if (connection === undefined || !connection.isOpen || isReplaced) {
                    throw new TypeError("only an open connection can start TLS");
                }

                // replace this end with a new one receiving into a new inbox, ending this one's readable
                const receiving = way === "toServer" ? "toClient" : "toServer";
                const fresh = SocketInbox.open(() => undefined);
                connection.ways[receiving].inbox = fresh;
                isReplaced = true;
                inbox.controller.close();
                closing.resolve();

                return this.#socketEnd({
                    connected: Promise.resolve(connection),
                    way,
                    inbox: fresh,
                    connect: { ...connect, secureTransport: "on" },
                });
            },
        };

        return socket;
    }

    /** Deliver socket bytes one way, in order, after their latency, or break the connection. */
    #deliverSocket(connection: SocketConnection, way: Direction, arrive: () => void): void {
        // break a connection across a partition or losing the bytes, which a node never does to itself
        const { address, client, server } = connection;
        if (
            this.#isSocketCut(address, client, server) ||
            this.#isSocketLost(address, client, server)
        ) {
            this.#breakSocket(connection);

            return;
        }

        // arrive after the latency, never before earlier bytes of the same way, unless it ended meanwhile
        const at = Math.max(this.#clock.now() + this.#latency(), connection.ways[way].arrival);
        connection.ways[way].arrival = at;
        const arrival = this.#clock.sleep(at - this.#clock.now()).then(() => {
            if (connection.isOpen) {
                arrive();
            }
        });
        void this.#transit(arrival);
    }

    /** End one way of a socket connection after its bytes in flight, then the other way. */
    #endSocket(connection: SocketConnection, way: Direction): void {
        // end each way of an open connection once
        const ending = connection.ways[way];
        if (!connection.isOpen || ending.state !== "open") {
            return;
        }
        ending.state = "ending";

        // close the receiver's readable once the earlier bytes arrived
        const other = way === "toServer" ? "toClient" : "toServer";
        this.#deliverSocket(connection, way, () => {
            ending.state = "ended";
            ending.inbox.controller.close();

            // end the other way as the peer, and settle every end's close once both ended
            if (connection.ways[other].state === "ended") {
                connection.isOpen = false;
                this.#socketConnections.delete(connection);
                for (const close of connection.closes) {
                    close();
                }
            } else {
                this.#endSocket(connection, other);
            }
        });
    }

    /** Break a socket connection at once at both ends, as a lost transport. */
    #breakSocket(connection: SocketConnection): void {
        // break once, failing both readables
        if (!connection.isOpen) {
            return;
        }
        const { client, server } = connection;
        const broken = refusal(`the connection ${client} ${server} broke`, "ECONNRESET");
        connection.isOpen = false;
        connection.broken = broken;
        this.#socketConnections.delete(connection);
        this.#record({ kind: "break", at: this.#clock.now(), client, server });

        // fail both readables, and settle every end's close
        connection.ways.toServer.inbox.controller.error(broken);
        connection.ways.toClient.inbox.controller.error(broken);
        for (const close of connection.closes) {
            close();
        }
    }

    /** Deliver a node's request to its origin's listener. */
    async #fetch(from: string, request: Request): Promise<Response> {
        // give the request up once its signal aborts, as a browser's fetch does
        const { signal } = request;
        const { promise: aborted, reject } = Promise.withResolvers<never>();
        const abort = () => reject(signal.reason);
        if (signal.aborted) {
            abort();
        } else {
            signal.addEventListener("abort", abort, { once: true });
        }

        // stop listening once the round trip ends either way
        try {
            return await Promise.race([this.#roundTrip(from, request), aborted]);
        } finally {
            signal.removeEventListener("abort", abort);
        }
    }

    /** Send a request to its endpoint and its response back, each way through the network. */
    async #roundTrip(from: string, request: Request): Promise<Response> {
        // hand the listener a copy of the request, refusing an upgrade answered to a plain request
        const endpoint = await this.#reach(from, request);
        const answer = await endpoint.answer(new Request(request));
        if (answer.kind === "socket") {
            throw new TypeError(`${new URL(request.url).origin} upgraded a plain request`);
        }

        // send the response back after the latency, losing it to a partition or a node that left
        await this.#clock.sleep(this.#latency(), request.signal);
        const { origin } = new URL(request.url);
        const isCut = this.#isCut(from, endpoint.node);
        if (isCut || this.#resolve(new URL(request.url)) !== endpoint) {
            const target = `${request.method} ${request.url}`;
            this.#record({ kind: "loss", at: this.#clock.now(), from, target });
            throw refusal(`fetch failed: the response from ${origin} was lost`, "ConnectionClosed");
        }

        // hand the node a copy of the response whose body fails once the request aborts, as fetch does
        const { body, status, statusText, headers } = answer.response;
        const delivered =
            body === null
                ? null
                : body.pipeThrough(new TransformStream(), { signal: request.signal });

        return new Response(delivered, { status, statusText, headers });
    }

    /** Open a node's socket to a listener. */
    #websocket(
        from: string,
        url: string,
        protocols: string | readonly string[] | undefined,
    ): SimulatedWebSocket {
        // make the client's socket, whose sends and close cross the network once it opens
        let connection: Connection | undefined;
        const socket = new SimulatedWebSocket(
            url,
            (message) => {
                if (connection !== undefined) {
                    this.#deliver(connection, "toServer", () => connection?.end.message(message));
                }
            },
            (code, reason) => {
                if (connection === undefined) {
                    socket.closed(code, reason);
                } else {
                    this.#close(connection, "toServer", code, reason);
                }
            },
        );

        // ask the listener for the upgrade over HTTP
        const offered = typeof protocols === "string" ? [protocols] : [...(protocols ?? [])];
        const target = new URL(url);
        target.protocol = target.protocol === "wss:" ? "https:" : "http:";
        const request = new Request(target.href, {
            headers: {
                upgrade: "websocket",
                connection: "Upgrade",
                ...(offered.length === 0 ? {} : { "sec-websocket-protocol": offered.join(", ") }),
            },
        });

        // open the socket on an accepted upgrade, and fail it otherwise
        const opening = async () => {
            // ask the endpoint, discarding a plain answer's body as a refused client does
            const endpoint = await this.#reach(from, request);
            const answer = await endpoint.answer(request);
            if (answer.kind === "response") {
                await answer.response.body?.cancel();
            }

            // fail the socket on a plain answer, or once it closed while connecting
            if (answer.kind === "response" || socket.readyState !== SimulatedWebSocket.CONNECTING) {
                socket.closed(ABNORMAL_CLOSURE, "");

                return;
            }

            // open the connection at both ends
            connection = {
                client: from,
                server: endpoint.node,
                address: SimulatedNetwork.#address(target),
                socket,
                end: answer.end,
                arrivals: { toServer: 0, toClient: 0 },
                isOpen: true,
            };
            this.#connections.add(connection);
            const opened = connection;
            answer.end.open({
                send: (message) =>
                    this.#deliver(opened, "toClient", () => opened.socket.received(message)),
                close: (code, reason) => this.#close(opened, "toClient", code, reason),
            });
            socket.opened(answer.protocol);
        };
        void this.#transit(opening().catch(() => socket.closed(ABNORMAL_CLOSURE, "")));

        return socket;
    }

    /** Deliver a message one way, in order, after its latency, or break its connection. */
    #deliver(connection: Connection, direction: Direction, arrive: () => void): void {
        // skip a closed connection, and break one across a partition or losing the message
        const isCut = this.#isCut(connection.client, connection.server);
        if (!connection.isOpen) {
            return;
        } else if (isCut || this.#isLost(connection.client, connection.server)) {
            this.#break(connection);

            return;
        }

        // arrive after the latency, never before an earlier message of the same direction
        const at = Math.max(this.#clock.now() + this.#latency(), connection.arrivals[direction]);
        connection.arrivals[direction] = at;
        const arrival = this.#clock.sleep(at - this.#clock.now()).then(() => {
            if (connection.isOpen) {
                arrive();
            }
        });
        void this.#transit(arrival);
    }

    /** Close a connection from one end, the other end learning of it after the latency. */
    #close(connection: Connection, direction: Direction, code: number, reason: string): void {
        this.#deliver(connection, direction, () => {
            // end the connection at both ends
            connection.isOpen = false;
            this.#connections.delete(connection);
            connection.end.close(code, reason);
            connection.socket.closed(code, reason);
        });
    }

    /** Break a connection at once at both ends, as a lost transport. */
    #break(connection: Connection): void {
        // break once
        if (!connection.isOpen) {
            return;
        }
        connection.isOpen = false;
        this.#connections.delete(connection);

        // record it, and close both ends abnormally
        const { client, server } = connection;
        this.#record({ kind: "break", at: this.#clock.now(), client, server });
        connection.end.close(ABNORMAL_CLOSURE, "");
        connection.socket.closed(ABNORMAL_CLOSURE, "");
    }

    /** Find a request's listener after the latency, refusing an unserved, cut off or lost one. */
    async #reach(from: string, request: Request): Promise<Endpoint> {
        // refuse an origin nothing serves, a node cut off from it, and a lost request
        const url = new URL(request.url);
        const { origin } = url;
        const endpoint = this.#resolve(url);
        const target = `${request.method} ${request.url}`;
        const at = this.#clock.now();
        if (endpoint === undefined) {
            this.#record({ kind: "refusal", at, from, target, reason: "unserved" });
            throw refusal(`fetch failed: nothing serves ${origin}`, "ConnectionRefused");
        } else if (this.#isCut(from, endpoint.node)) {
            this.#record({ kind: "refusal", at, from, target, reason: "partitioned" });
            throw refusal(
                `fetch failed: ${from} cannot reach ${endpoint.node}`,
                "FailedToOpenSocket",
            );
        } else if (this.#isLost(from, endpoint.node)) {
            this.#record({ kind: "loss", at, from, target });
            throw refusal(`fetch failed: the request to ${origin} was lost`, "ConnectionClosed");
        }

        // arrive after the latency
        await this.#clock.sleep(this.#latency(), request.signal);
        this.#record({ kind: "request", at: this.#clock.now(), from, target });

        return endpoint;
    }

    /** Report whether the network cuts a socket connection, never a Unix socket's. */
    #isSocketCut(key: string, left: string, right: string): boolean {
        return !key.startsWith(UNIX_KEY) && this.#isCut(left, right);
    }

    /** Draw whether a socket delivery is lost, never one over a Unix socket. */
    #isSocketLost(key: string, left: string, right: string): boolean {
        return !key.startsWith(UNIX_KEY) && this.#isLost(left, right);
    }

    /** Draw whether a delivery between two nodes is lost, never one a node sends itself. */
    #isLost(left: string, right: string): boolean {
        const { drop } = this.#faults;

        return (
            left !== right &&
            drop !== undefined &&
            Random.int(this.#random, 1_000_000) < drop * 1_000_000
        );
    }

    /** Draw a delivery's latency. */
    #latency(): number {
        const { delay } = this.#faults;

        return delay === undefined
            ? 0
            : delay.least + Random.int(this.#random, delay.most - delay.least + 1);
    }

    /** Count a delivery as sent, and as delivered once it settles. */
    #transit<Settled>(delivery: Promise<Settled>): Promise<Settled> {
        // count it as sent, and as delivered once it settles
        this.#sent += 1;
        const settle = () => {
            this.#delivered += 1;
        };
        void delivery.then(settle, settle);

        return delivery;
    }

    /** Find a URL's listener at its exact host, else the nearest wildcard above it (RFC 4592). */
    #resolve(url: URL): Endpoint | undefined {
        // take the exact host first
        const port = url.port === "" ? DEFAULT_PORTS[url.protocol] : url.port;
        const exact = this.#endpoints.get(`${url.hostname}:${port}`);
        if (exact !== undefined) {
            return exact;
        }

        // take the wildcard of the nearest enclosing name, which stands for one or more labels
        const labels = url.hostname.split(".");
        for (let index = 1; index < labels.length; index += 1) {
            const wildcard = this.#endpoints.get(`*.${labels.slice(index).join(".")}:${port}`);
            if (wildcard !== undefined) {
                return wildcard;
            }
        }

        // resolve localhost and the names below it to the loopback, as RFC 6761 6.3 reserves them
        const isLoopback = url.hostname === "localhost" || url.hostname.endsWith(".localhost");

        return isLoopback
            ? (this.#endpoints.get(`127.0.0.1:${port}`) ?? this.#endpoints.get(`localhost:${port}`))
            : undefined;
    }

    /** Read a URL's host name and port, the scheme's default port without one. */
    static #address(url: URL): string {
        return `${url.hostname}:${url.port === "" ? DEFAULT_PORTS[url.protocol] : url.port}`;
    }

    /** Key a pair of nodes alike in either order. */
    static #pair(left: string, right: string): string {
        return [left, right].toSorted().join(" ");
    }
}

/** Fail a connection as the host's network does: a TypeError with Bun's error code. */
function refusal(message: string, code: string): TypeError {
    return Object.assign(new TypeError(message), { code });
}

/** Decode a response's gzip or deflate encoding, dropping the encoding and length it states. */
function decoded(response: Response): Response {
    // keep a response without a body or an encoding the web's streams decode
    const encoding = response.headers.get("content-encoding");
    if (response.body === null || (encoding !== "gzip" && encoding !== "deflate")) {
        return response;
    }

    // decode the body, and describe the decoded bytes
    const headers = new Headers(response.headers);
    headers.delete("content-encoding");
    headers.delete("content-length");
    const body = response.body.pipeThrough(new DecompressionStream(encoding));

    return new Response(body, {
        status: response.status,
        statusText: response.statusText,
        headers,
    });
}

/** Key a socket address on the simulated network: its host and port, or its Unix socket's path. */
function socketKey(address: SocketAddress): string {
    return "unix" in address ? `${UNIX_KEY}${address.unix}` : `${address.hostname}:${address.port}`;
}
