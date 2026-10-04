import { serve, type Server, type ServerWebSocket, type TLSOptions } from "bun";
import { MAX_FRAME_BYTES, type Session, TUNNEL_PROTOCOL } from "../session/index.ts";
import type { Admission, Relay } from "../server/index.ts";

/** A tunnel connection's WebSocket: its host's admission, and its session once open. */
interface Socket {
    /** The admitted host. */
    readonly admission: Admission;
    /** The session over the socket, once open. */
    session?: Session;
}

/** Where a Bun process listens for a relay, with TLS unless a proxy in front terminates it. */
export interface RelayListener {
    /** The network interface. */
    readonly hostname: string;
    /** The port. */
    readonly port: number;
    /** The certificate and key the listener presents. */
    readonly tls?: TLSOptions;
}

/** A relay's listener in a Bun process: its names and its hosts' tunnels on one port. */
export class BunRelay {
    /** The relay the listener serves. */
    readonly relay: Relay;
    /** The listener. */
    readonly #listener: Server<Socket>;

    /** Listen for a relay's names and tunnels. */
    private constructor(relay: Relay, listener: RelayListener) {
        this.relay = relay;
        this.#listener = serve<Socket>({
            hostname: listener.hostname,
            port: listener.port,
            ...(listener.tls === undefined ? {} : { tls: listener.tls }),
            fetch: (request, bun) => this.#serve(request, bun),
            websocket: {
                maxPayloadLength: MAX_FRAME_BYTES,
                open: (socket) => {
                    socket.data.session = relay.attach(socket.data.admission, {
                        send: (message) => void socket.send(message),
                        close: () => socket.close(),
                    });
                },
                message: (socket, message) => BunRelay.#receive(socket, message),
                close: (socket) => {
                    const { session } = socket.data;
                    if (session !== undefined) {
                        relay.detach(socket.data.admission.hostId, session);
                    }
                },
            },
        });
    }

    /** Listen for a relay on a Bun process's port. */
    static listen(relay: Relay, listener: RelayListener): BunRelay {
        return new BunRelay(relay, listener);
    }

    /** The URL hosts open their tunnels at. */
    get url(): string {
        return this.relay.url;
    }

    /** The port the listener bound. */
    get port(): number {
        const port = this.#listener.port;
        if (port === undefined) {
            throw new TypeError("the relay listens on no port");
        }

        return port;
    }

    /** Stop listening, ending every tunnel. */
    async close(): Promise<void> {
        await this.#listener.stop(true);
    }

    /** Upgrade a host's admitted tunnel with the tunnel protocol, and answer every other request. */
    async #serve(request: Request, bun: Server<Socket>): Promise<Response | undefined> {
        // answer a request for a name or a refused tunnel
        const routed = await this.relay.route(request);
        if (routed instanceof Response) {
            return routed;
        }

        // upgrade the admitted host's connection
        const headers = { "sec-websocket-protocol": TUNNEL_PROTOCOL };

        return bun.upgrade(request, { data: { admission: routed }, headers })
            ? undefined
            : new Response(null, { status: 426 });
    }

    /** Feed a connection's message to its session, closing connections that send text. */
    static #receive(socket: ServerWebSocket<Socket>, message: string | Uint8Array): void {
        if (typeof message === "string") {
            socket.close();

            return;
        }
        socket.data.session?.receive(message);
    }
}
