import { TUNNEL_PROTOCOL } from "../session/index.ts";
import type { RelayServer } from "../server/index.ts";

/** The server end of a WebSocket workerd accepts in a Durable Object. */
interface ServerSocket {
    /** Take the connection's events in this object. */
    accept(): void;
    /** Send one binary message. */
    send(message: Uint8Array): void;
    /** Close the connection. */
    close(): void;
    /** Receive a message, or the connection's end. */
    addEventListener(
        type: "message" | "close" | "error",
        listener: (event: { readonly data?: unknown }) => void,
    ): void;
}

/** The pair of WebSocket ends workerd creates for an upgrade: the client's, then the server's. */
declare const WebSocketPair: new () => { readonly 0: unknown; readonly 1: ServerSocket };

/** What workerd answers an upgrade with: the switch, and the client's end of the WebSocket. */
interface Upgrade extends ResponseInit {
    /** The client's end of the WebSocket. */
    readonly webSocket: unknown;
}

/** A relay in a Durable Object: its names, and its hosts' tunnels over WebSockets the object accepts and keeps open. */
export class WorkerdRelay {
    /** The relay the object serves. */
    readonly server: RelayServer;
    /** The server ends of the open tunnels. */
    readonly #sockets = new Set<ServerSocket>();

    /** Serve a relay from the object holding it. */
    constructor(server: RelayServer) {
        this.server = server;
    }

    /** Accept a host's admitted tunnel with the tunnel protocol, and answer every other request. */
    async fetch(request: Request): Promise<Response> {
        // answer a request for a name or a refused tunnel
        const routed = await this.server.route(request);
        if (routed instanceof Response) {
            return routed;
        }

        // run the host's session over the server's end
        const pair = new WebSocketPair();
        const socket = pair[1];
        socket.accept();
        this.#sockets.add(socket);
        const session = this.server.attach(routed, {
            send: (message) => socket.send(message),
            close: () => socket.close(),
        });

        // read binary frames and close a connection that sends text
        socket.addEventListener("message", ({ data }) => {
            // read a binary frame
            if (data instanceof ArrayBuffer) {
                session.receive(new Uint8Array(data));
            }
            // refuse a text frame
            else {
                socket.close();
            }
        });

        // forget the socket once it closes
        socket.addEventListener("close", () => {
            this.#sockets.delete(socket);
            this.server.detach(routed.hostId, session);
        });

        // switch protocols, handing the client's end to the host
        const upgrade: Upgrade = {
            status: 101,
            headers: { "sec-websocket-protocol": TUNNEL_PROTOCOL },
            webSocket: pair[0],
        };

        return new Response(null, upgrade);
    }

    /** End every tunnel. */
    async close(): Promise<void> {
        for (const socket of this.#sockets) {
            socket.close();
        }
        this.#sockets.clear();
    }
}
