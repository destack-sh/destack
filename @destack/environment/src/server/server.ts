import type { SocketListener, SocketListenOptions } from "../network/socket/socket.ts";

/** A WebSocket a listener accepted, with its attachment. */
export interface ServerSocket<Attachment> {
    /** The value the listener stored beside the socket as it accepted it. */
    readonly attachment: Attachment;
    /** Send a message to the client. */
    readonly send: (message: string | Uint8Array<ArrayBuffer>) => void;
    /** Close the socket. */
    readonly close: (code?: number, reason?: string) => void;
}

/** What a listener does with its accepted sockets' events. */
export interface ServerSocketHandler<Attachment> {
    /** The most bytes in one message. */
    readonly maxPayloadLength?: number;
    /** Take a socket as it opens. */
    readonly open?: (socket: ServerSocket<Attachment>) => void;
    /** Take one message a socket received. */
    readonly message: (socket: ServerSocket<Attachment>, message: string | Uint8Array) => void;
    /** Take a socket's close. */
    readonly close?: (socket: ServerSocket<Attachment>, code: number, reason: string) => void;
}

/** An open listener, answering requests at its URL. */
export interface Listener<Attachment> {
    /** The URL it listens at. */
    readonly url: URL;
    /** Accept a request's WebSocket upgrade with an attachment, answering whether it took it. */
    readonly upgrade: (
        request: Request,
        options: { readonly attachment: Attachment; readonly headers?: Record<string, string> },
    ) => boolean;
    /** Stop listening, closing open connections when asked. */
    readonly stop: (isClosingActive?: boolean) => Promise<void>;
}

/** What a listener answers. */
export interface ServeOptions<Attachment> {
    /** The host name it listens at. */
    readonly hostname?: string;
    /** The port it listens at, a free one for zero. */
    readonly port?: number;
    /** The Unix socket it listens at instead of a port. */
    readonly unix?: string;
    /** Answer a request, or nothing once its upgrade is accepted. */
    readonly fetch: (
        request: Request,
        listener: Listener<Attachment>,
    ) => Response | undefined | Promise<Response | undefined>;
    /** Take the accepted sockets' events. */
    readonly websocket?: ServerSocketHandler<Attachment>;
}

/** The accepting of connections: requests and WebSockets, and stream sockets. */
export interface Server {
    /** Accept requests and WebSockets, as `Bun.serve` does. */
    readonly http: <Attachment>(options: ServeOptions<Attachment>) => Listener<Attachment>;
    /** Accept connections at a host name and port or a Unix socket's path, as `Bun.listen` does. */
    readonly socket: (options: SocketListenOptions) => Promise<SocketListener>;
}
