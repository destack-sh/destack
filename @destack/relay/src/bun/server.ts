import { serve, type Server, type ServerWebSocket, type TLSOptions } from "bun";
import {
    MAX_FRAME_BYTES,
    PING_MESSAGE,
    PONG_MESSAGE,
    type Session,
    TUNNEL_PROTOCOL,
} from "../session/index.ts";
import type { Admission, MemoryTunnelHost, Relay } from "../server/index.ts";

/** A tunnel connection's WebSocket: its machine's admission, and its session once open. */
interface Socket {
    /** The admitted machine. */
    readonly admission: Admission;
    /** The session over the socket, once open. */
    session?: Session;
}

/** Where a Bun process listens for a relay, with TLS unless a proxy in front terminates it. */
export interface BunRelayListener {
    /** The network interface. */
    readonly hostname: string;
    /** The port. */
    readonly port: number;
    /** The certificate and key the listener presents. */
    readonly tls?: TLSOptions;
}

/** A relay's listener in a Bun process: its names, and its machines' tunnels kept in the process's memory, on one port. */
export class BunRelay {
    /** The relay the listener serves. */
    readonly relay: Relay;
    /** The listener. */
    readonly #listener: Server<Socket>;

    /** Listen for a relay's names and tunnels. */
    private constructor(relay: Relay, tunnels: MemoryTunnelHost, listener: BunRelayListener) {
        this.relay = relay;
        this.#listener = serve<Socket>({
            hostname: listener.hostname,
            port: listener.port,
            ...(listener.tls === undefined ? {} : { tls: listener.tls }),
            fetch: (request, bun) => this.#serve(request, bun),
            websocket: {
                maxPayloadLength: MAX_FRAME_BYTES,
                open: (socket) => {
                    socket.data.session = tunnels.attach(relay, socket.data.admission, {
                        send: (message) => void socket.send(message),
                        close: () => socket.close(),
                    });
                },
                message: (socket, message) => BunRelay.#receive(socket, message),
                close: (socket) => {
                    const { session } = socket.data;
                    if (session !== undefined) {
                        tunnels
                            .detach(socket.data.admission.machineId, session)
                            .catch(relay.report);
                    }
                },
            },
        });
    }

    /** Listen for a relay on a Bun process's port, keeping its machines' tunnels in memory. */
    static listen(relay: Relay, tunnels: MemoryTunnelHost, listener: BunRelayListener): BunRelay {
        return new BunRelay(relay, tunnels, listener);
    }

    /** The URL machines open their tunnels at. */
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

    /** Upgrade a machine's admitted tunnel with the tunnel protocol, and answer every other request. */
    async #serve(request: Request, bun: Server<Socket>): Promise<Response | undefined> {
        // answer a request for a name or a refused tunnel
        const routed = await this.relay.route(request);
        if (routed instanceof Response) {
            return routed;
        }

        // upgrade the admitted machine's connection
        const headers = { "sec-websocket-protocol": TUNNEL_PROTOCOL };

        return bun.upgrade(request, { data: { admission: routed }, headers })
            ? undefined
            : new Response(null, { status: 426 });
    }

    /** Feed a connection's binary message to its session, answer a liveness probe, and close a connection sending other text. */
    static #receive(socket: ServerWebSocket<Socket>, message: string | Uint8Array): void {
        // read a binary frame
        if (typeof message !== "string") {
            socket.data.session?.receive(message);
        }
        // answer a liveness probe
        else if (message === PING_MESSAGE) {
            socket.send(PONG_MESSAGE);
        }
        // refuse other text
        else {
            socket.close();
        }
    }
}
