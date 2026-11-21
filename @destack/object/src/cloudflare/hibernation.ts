import { WebSocketClosure } from "@destack/environment";
import { type HibernationHost, type HostSocket, HostSocketAttachment } from "../workload/socket.ts";

/** The server end of a WebSocket workerd leaves open for a Durable Object across hibernation. */
export interface HibernatingSocket {
    /** Send one message. */
    readonly send: (message: string | ArrayBuffer | Uint8Array) => void;
    /** Accept the WebSocket in this isolate, without hibernation. */
    readonly accept: () => void;
    /** Close the connection with a code and reason. */
    readonly close: (code?: number, reason?: string) => void;
    /** Attach a value to the WebSocket across hibernation. */
    readonly serializeAttachment: (value: unknown) => void;
    /** Read the value stored with the WebSocket. */
    readonly deserializeAttachment: () => unknown;
}

/** The globals workerd supplies to every module. */
declare global {
    /** The pair of WebSocket ends workerd creates for an upgrade, the client's first. */
    const WebSocketPair: new () => { readonly 0: unknown; readonly 1: HibernatingSocket };
}

/** What workerd answers an upgrade with: the switch, and the client's end of the WebSocket. */
export interface Upgrade extends ResponseInit {
    /** The client's end of the WebSocket. */
    readonly webSocket: unknown;
}

/** The members of a Durable Object's state its hibernating WebSockets use. */
export interface HibernationState {
    /** Accept a WebSocket that stays open across hibernation under tags. */
    readonly acceptWebSocket: (socket: HibernatingSocket, tags: readonly string[]) => void;
    /** Read the tags a WebSocket was accepted under. */
    readonly getTags: (socket: HibernatingSocket) => string[];
}

/** A Durable Object's WebSocket hibernation, keeping one tag's sockets for a hibernation host. */
export class DurableObjectHibernation {
    /** The object's state. */
    readonly #state: HibernationState;
    /** The tag of the sockets, beside other tags the object accepts sockets under. */
    readonly #tag: string;
    /** The sockets as the host keeps them, rebuilt after hibernation from their attachments. */
    readonly #kept = new WeakMap<HibernatingSocket, HostSocket>();

    /** Keep an object's sockets under a tag. */
    constructor(state: HibernationState, tag: string) {
        this.#state = state;
        this.#tag = tag;
    }

    /** Report whether the object accepted a socket under the tag. */
    has(socket: HibernatingSocket): boolean {
        return this.#state.getTags(socket).includes(this.#tag);
    }

    /** Accept the WebSocket an instance accepted in its answer, keeping it at the host. */
    accept(host: HibernationHost, answer: Response): Response {
        // keep the socket's acceptance beside the server's end across hibernation
        const { attachment, headers } = host.accept(answer);
        const pair = new WebSocketPair();
        this.#state.acceptWebSocket(pair[1], [this.#tag]);
        pair[1].serializeAttachment(attachment);
        this.#keep(host, pair[1], attachment);

        // answer the upgrade with the client's end
        const upgrade: Upgrade = {
            status: 101,
            webSocket: pair[0],
            ...(headers === undefined ? {} : { headers }),
        };

        return new Response(null, upgrade);
    }

    /** Hand a socket's message to the host, which delivers each in order. */
    message(host: HibernationHost, socket: HibernatingSocket, message: ArrayBuffer | string): void {
        const kept = this.#socket(host, socket);
        host.websocket.message(
            kept,
            typeof message === "string" ? message : new Uint8Array(message),
        );
    }

    /** Hand a socket's close to the host, forgetting the socket, and answer the close. */
    close(
        host: Pick<HibernationHost, "websocket">,
        socket: HibernatingSocket,
        code: number,
        reason: string,
    ): void {
        // tell the host, which forgets the socket
        const kept = this.#socket(host, socket);
        this.#kept.delete(socket);
        host.websocket.close?.(kept, code, reason);

        // answer the peer's close, which waits for it to finish closing (RFC 6455 7.1.2)
        socket.close(WebSocketClosure.sendable(code), reason);
    }

    /** Read a socket as the host keeps it, rebuilding it after hibernation. */
    #socket(host: Pick<HibernationHost, "websocket">, socket: HibernatingSocket): HostSocket {
        return (
            this.#kept.get(socket) ??
            this.#keep(host, socket, HostSocketAttachment.parse(socket.deserializeAttachment()))
        );
    }

    /** Keep a hibernating socket at the host, its sends and close going to the socket itself. */
    #keep(
        host: Pick<HibernationHost, "websocket">,
        socket: HibernatingSocket,
        attachment: HostSocketAttachment,
    ): HostSocket {
        // register the socket at the host, remembering it until it closes
        const kept: HostSocket = {
            attachment,
            send: async (message) => socket.send(message),
            close: (code, reason) => socket.close(code, reason),
        };
        host.websocket.open?.(kept);
        this.#kept.set(socket, kept);

        return kept;
    }
}
