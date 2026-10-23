import type {
    Listener,
    Network,
    ServeOptions,
    ServerSocket,
    ServerSocketHandler,
} from "../network/network.ts";
import { Random } from "../random/random.ts";
import type { VirtualClock } from "./clock.ts";
import type { SimulationEvent } from "./simulation.ts";
import { ABNORMAL_CLOSURE, NORMAL_CLOSURE, SimulatedWebSocket } from "./socket.ts";

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
    /** The listener's host name and port. */
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

        // keep the attachment and the chosen subprotocol until the answer
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
    /** The pairs of nodes that cannot reach each other, as `a b` keys. */
    readonly #partitions = new Set<string>();
    /** The deliveries in flight. */
    readonly #deliveries = new Set<object>();
    /** The next free port. */
    #port = FIRST_DYNAMIC_PORT;
    /** The faults every delivery meets. */
    #faults: NetworkFaults = {};
    /** The simulation's clock, which latency waits on. */
    readonly #clock: VirtualClock;
    /** The simulation's randomness, which latency and loss draw from. */
    readonly #random: Random;
    /** Record what happens, in order. */
    readonly #record: (event: SimulationEvent) => void;

    /** Connect the network to its simulation's clock, randomness and trace. */
    constructor(clock: VirtualClock, random: Random, record: (event: SimulationEvent) => void) {
        this.#clock = clock;
        this.#random = random;
        this.#record = record;
    }

    /** The deliveries in flight. */
    get inFlight(): number {
        return this.#deliveries.size;
    }

    /** Read the network a node reaches other origins through and listens on, without TLS. */
    of(node: string): Network {
        return {
            fetch: (request) => this.#track(this.#fetch(node, request)),
            connect: (url, protocols) => this.#connect(node, url, protocols),
            serve: (options) => this.#serve(node, options),
        };
    }

    /** Cut the network between two nodes, both ways, breaking their connections. */
    partition(left: string, right: string): void {
        // cut the pair
        const pair = SimulatedNetwork.#pair(left, right);
        this.#partitions.add(pair);
        this.#record({ kind: "partition", at: this.#clock.now(), detail: pair });

        // break its connections
        for (const connection of this.#connections) {
            if (SimulatedNetwork.#pair(connection.client, connection.server) === pair) {
                this.#break(connection);
            }
        }
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
        const detail = `${left ?? "*"} ${right ?? "*"}`;
        this.#record({ kind: "heal", at: this.#clock.now(), detail });
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
        const port = options.port === undefined || options.port === 0 ? this.#port : options.port;
        this.#port += port === this.#port ? 1 : 0;
        const url = new URL(`http://${options.hostname ?? "localhost"}:${port}/`);
        const address = SimulatedNetwork.#address(url);
        if (this.#endpoints.has(address)) {
            throw new TypeError(`${address} is already served`);
        }

        // answer as the node until the listener stops, closing its connections when asked
        const listener = new SimulatedListener(url, options, (isClosingActive) => {
            this.#endpoints.delete(address);
            for (const connection of this.#connections) {
                if (isClosingActive && connection.address === address) {
                    this.#close(connection, "toClient", GOING_AWAY, "");
                }
            }
        });
        this.#endpoints.set(address, { node, answer: (request) => listener.answer(request) });

        return listener;
    }

    /** Deliver a node's request to its origin's listener. */
    async #fetch(from: string, request: Request): Promise<Response> {
        // refuse an upgrade answered to a plain request
        const endpoint = await this.#reach(from, request);
        const answer = await endpoint.answer(request);
        if (answer.kind === "socket") {
            throw new TypeError(`${new URL(request.url).origin} upgraded a plain request`);
        }

        return answer.response;
    }

    /** Open a node's socket to a listener. */
    #connect(
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
        void this.#track(opening().catch(() => socket.closed(ABNORMAL_CLOSURE, "")));

        return socket;
    }

    /** Deliver a message one way, in order, after its latency, or break its connection. */
    #deliver(connection: Connection, direction: Direction, arrive: () => void): void {
        // skip a closed connection, and break one across a partition or losing the message
        const isCut = this.#partitions.has(
            SimulatedNetwork.#pair(connection.client, connection.server),
        );
        if (!connection.isOpen) {
            return;
        } else if (isCut || this.#isLost()) {
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
        void this.#track(arrival);
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
        const detail = `${connection.client} ${connection.server}`;
        this.#record({ kind: "break", at: this.#clock.now(), detail });
        connection.end.close(ABNORMAL_CLOSURE, "");
        connection.socket.closed(ABNORMAL_CLOSURE, "");
    }

    /** Find a request's listener after the latency, refusing an unserved, cut off or lost one. */
    async #reach(from: string, request: Request): Promise<Endpoint> {
        // refuse an origin nothing serves, a node cut off from it, and a lost request
        const url = new URL(request.url);
        const { origin } = url;
        const endpoint = this.#endpoints.get(SimulatedNetwork.#address(url));
        const detail = `${from} ${request.method} ${request.url}`;
        const at = this.#clock.now();
        if (endpoint === undefined) {
            this.#record({ kind: "refusal", at, detail: `${detail}: unserved` });
            throw new TypeError(`fetch failed: nothing serves ${origin}`);
        } else if (this.#partitions.has(SimulatedNetwork.#pair(from, endpoint.node))) {
            this.#record({ kind: "refusal", at, detail: `${detail}: partitioned` });
            throw new TypeError(`fetch failed: ${from} cannot reach ${endpoint.node}`);
        } else if (this.#isLost()) {
            this.#record({ kind: "loss", at, detail });
            throw new TypeError(`fetch failed: the request to ${origin} was lost`);
        }

        // arrive after the latency
        await this.#clock.sleep(this.#latency());
        this.#record({ kind: "request", at: this.#clock.now(), detail });

        return endpoint;
    }

    /** Draw whether a delivery is lost. */
    #isLost(): boolean {
        const { drop } = this.#faults;

        return drop !== undefined && Random.int(this.#random, 1_000_000) < drop * 1_000_000;
    }

    /** Draw a delivery's latency. */
    #latency(): number {
        const { delay } = this.#faults;

        return delay === undefined
            ? 0
            : delay.least + Random.int(this.#random, delay.most - delay.least + 1);
    }

    /** Count a delivery in flight until it settles. */
    #track<Settled>(delivery: Promise<Settled>): Promise<Settled> {
        // count it until it settles
        const token = {};
        this.#deliveries.add(token);
        const forget = () => this.#deliveries.delete(token);
        void delivery.then(forget, forget);

        return delivery;
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
