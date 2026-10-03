import { serve, type Server, type ServerWebSocket, type TLSOptions } from "bun";
import type { Identifier } from "@destack/schema";
import { Bearer } from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { TimerAlarm } from "@destack/service/control";
import { reportError } from "@destack/service/server";
import { MAX_FRAME_BYTES, type Session, TUNNEL_PROTOCOL } from "../session/index.ts";
import { type Admission, Relay, type RelayOptions, TUNNEL_PATH, Tunnel } from "../server/index.ts";

/** A tunnel connection's WebSocket: its host's admission, and its session once open. */
interface Socket {
    /** The admitted host. */
    readonly admission: Admission;
    /** The session over the socket, once open. */
    session?: Session;
}

/** Where and how a relay process serves. */
export interface RelayServerOptions extends Omit<RelayOptions, "tunnel"> {
    /** The relay's own origin, where hosts open their tunnels. */
    readonly origin: string;
    /** Where the process listens, with TLS unless a proxy in front terminates it. */
    readonly listener: {
        readonly hostname: string;
        readonly port: number;
        readonly tls?: TLSOptions;
    };
    /** The client certificate the relay presents to regions, which accept requests only with it. */
    readonly certificate?: { readonly cert: string; readonly key: string };
    /** Report a failure the relay answers to no one. */
    readonly report: (error: unknown) => void;
}

/** A relay in one process: its names and its hosts' tunnels on one listener, the tunnels kept in memory. */
export class RelayServer {
    /** The relay routing names. */
    readonly relay: Relay;
    /** The relay's own origin. */
    readonly #origin: URL;
    /** Report a failure the relay answers to no one. */
    readonly #report: (error: unknown) => void;
    /** The tunnel of each connected host. */
    readonly #tunnels = new Map<string, Tunnel>();
    /** Stops following the resolver. */
    readonly #stopping = new AbortController();
    /** The resolver's following. */
    readonly #following: Promise<void>;
    /** The listener. */
    readonly #server: Server<Socket>;

    /** Serve a relay as the options describe. */
    private constructor(options: RelayServerOptions) {
        // route names to the tunnels kept here, following the directory
        this.#origin = new URL(options.origin);
        this.#report = options.report;
        const certificate = options.certificate;
        this.relay = new Relay({
            ...options,
            tunnel: (hostId) => this.#tunnels.get(hostId),
            ...(certificate === undefined
                ? {}
                : { fetch: (request: Request) => fetch(request, { tls: certificate }) }),
        });
        this.#following = options.resolver.follow(this.#stopping.signal).catch(options.report);

        // serve names and tunnels
        this.#server = serve<Socket>({
            hostname: options.listener.hostname,
            port: options.listener.port,
            ...(options.listener.tls === undefined ? {} : { tls: options.listener.tls }),
            fetch: (request, server) => this.#serve(request, server),
            websocket: {
                maxPayloadLength: MAX_FRAME_BYTES,
                open: (socket) => this.#attach(socket),
                message: (socket, message) => this.#receive(socket, message),
                close: (socket) => void this.#detach(socket).catch(options.report),
            },
        });
    }

    /** Start a relay process. */
    static start(options: RelayServerOptions): RelayServer {
        return new RelayServer(options);
    }

    /** The URL hosts open their tunnels at. */
    get url(): string {
        return new URL(TUNNEL_PATH, this.#origin).href;
    }

    /** The port the listener bound. */
    get port(): number {
        const port = this.#server.port;
        if (port === undefined) {
            throw new TypeError("the relay listens on no port");
        }

        return port;
    }

    /** Stop serving: end every tunnel and stop following the directory. */
    async close(): Promise<void> {
        this.#stopping.abort();
        await this.#following;
        await this.#server.stop(true);
    }

    /** Open a host's tunnel at the relay's own origin, and forward every other request by its name. */
    async #serve(request: Request, server: Server<Socket>): Promise<Response | undefined> {
        // forward requests for names
        const url = new URL(request.url);
        const isTunnel = url.host === this.#origin.host && url.pathname === TUNNEL_PATH;
        if (!isTunnel) {
            return this.relay.fetch(request);
        }

        // admit the host by its offered token and answer with the tunnel protocol
        try {
            const admission = await this.relay.open(request);
            const headers = { "sec-websocket-protocol": TUNNEL_PROTOCOL };

            return server.upgrade(request, { data: { admission }, headers })
                ? undefined
                : new Response(null, { status: 426 });
        } catch (error) {
            const reported = reportError(error);

            return new Response(null, { status: reported.status });
        }
    }

    /** Carry a session over an opened connection, the newest of its host's tunnel. */
    #attach(socket: ServerWebSocket<Socket>): void {
        // find or keep the host's tunnel, renewing only the host's own tokens
        const { hostId, lapsesAt } = socket.data.admission;
        const tunnel =
            this.#tunnels.get(hostId) ??
            new Tunnel(hostId, {
                verify: (request) => this.#renew(hostId, request),
                alarm: new TimerAlarm(() => void tunnel.lapse().catch(this.#report)),
                report: this.#report,
            });
        this.#tunnels.set(hostId, tunnel);

        // carry the session over the socket
        socket.data.session = tunnel.attach(
            { send: (message) => void socket.send(message), close: () => socket.close() },
            lapsesAt,
        );
    }

    /** Feed a connection's message to its session, closing connections that send text. */
    #receive(socket: ServerWebSocket<Socket>, message: string | Uint8Array): void {
        if (typeof message === "string") {
            socket.close();

            return;
        }
        socket.data.session?.receive(message);
    }

    /** Forget a closed connection, and the host's tunnel once no connection of it remains. */
    async #detach(socket: ServerWebSocket<Socket>): Promise<void> {
        // find the socket's tunnel and session
        const { hostId } = socket.data.admission;
        const tunnel = this.#tunnels.get(hostId);
        const session = socket.data.session;
        if (tunnel === undefined || session === undefined) {
            return;
        }
        // forget the session, and the tunnel without connections
        await tunnel.detach(session);
        if (tunnel.isEmpty) {
            this.#tunnels.delete(hostId);
        }
    }

    /** Admit a renewal of a connection of a host's own tunnel, answering when its new token lapses. */
    async #renew(hostId: Identifier<"host">, request: Request): Promise<number> {
        // admit only the host's own token, renewed in the bearer header
        const admission = await this.relay.admit(Bearer.require(request.headers));
        if (admission.hostId !== hostId) {
            throw new ServiceError("FORBIDDEN", { message: "hosts only renew their own tunnels" });
        }

        return admission.lapsesAt;
    }
}
