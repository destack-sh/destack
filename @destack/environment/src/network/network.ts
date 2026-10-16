import { fetchBun } from "../bun/fetch.ts";
import { serveBun } from "../bun/listener.ts";

/** A TLS client certificate a request presents (RFC 8705). */
export interface ClientCertificate {
    /** The certificate chain in PEM. */
    readonly certificate: string;
    /** The private key in PEM. */
    readonly privateKey: string;
}

/** How a request is sent. */
export interface FetchOptions {
    /** The client certificate to present, for an mTLS origin. */
    readonly certificate?: ClientCertificate;
}

/** A WebSocket a client opened, its binary messages arriving as ArrayBuffers. */
export interface ClientSocket {
    /** The URL it connects to. */
    readonly url: string;
    /** The subprotocol the server chose. */
    readonly protocol: string;
    /** The connection's state, from connecting at 0 to closed at 3. */
    readonly readyState: number;
    /** Send a message. */
    readonly send: (message: string | Uint8Array<ArrayBuffer>) => void;
    /** Close the connection. */
    readonly close: (code?: number, reason?: string) => void;
    /** Take the open, a failure, a message or the close. */
    readonly addEventListener: {
        (
            type: "open" | "error",
            listener: (event: Event) => void,
            options?: AddEventListenerOptions,
        ): void;
        (
            type: "message",
            listener: (event: MessageEvent) => void,
            options?: AddEventListenerOptions,
        ): void;
        (
            type: "close",
            listener: (event: CloseEvent) => void,
            options?: AddEventListenerOptions,
        ): void;
    };
}

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

/** The network an environment reaches other origins through and listens on. */
export interface Network {
    /** Send a request and answer its response. */
    readonly fetch: (request: Request, options?: FetchOptions) => Promise<Response>;
    /** Open a WebSocket to a URL, offering subprotocols. */
    readonly connect: (url: string, protocols?: string | readonly string[]) => ClientSocket;
    /** Listen for requests and WebSockets. */
    readonly serve: <Attachment>(options: ServeOptions<Attachment>) => Listener<Attachment>;
}

/** The platform's WebSocket as a client socket. */
class SystemSocket implements ClientSocket {
    /** The platform's socket. */
    readonly #socket: WebSocket;

    /** Wrap the platform's socket, taking binary messages as ArrayBuffers. */
    constructor(socket: WebSocket) {
        this.#socket = socket;
        this.#socket.binaryType = "arraybuffer";
    }

    /** The URL it connects to. */
    get url(): string {
        return this.#socket.url;
    }

    /** The subprotocol the server chose. */
    get protocol(): string {
        return this.#socket.protocol;
    }

    /** The connection's state. */
    get readyState(): number {
        return this.#socket.readyState;
    }

    /** Send a message. */
    send(message: string | Uint8Array<ArrayBuffer>): void {
        this.#socket.send(message);
    }

    /** Close the connection. */
    close(code?: number, reason?: string): void {
        this.#socket.close(code, reason);
    }

    /** Take the open or a failure. */
    addEventListener(
        type: "open" | "error",
        listener: (event: Event) => void,
        options?: AddEventListenerOptions,
    ): void;
    /** Take a message. */
    addEventListener(
        type: "message",
        listener: (event: MessageEvent) => void,
        options?: AddEventListenerOptions,
    ): void;
    /** Take the close. */
    addEventListener(
        type: "close",
        listener: (event: CloseEvent) => void,
        options?: AddEventListenerOptions,
    ): void;
    /** Take an event. */
    addEventListener(
        type: "open" | "error" | "message" | "close",
        listener:
            | ((event: MessageEvent) => void)
            | ((event: CloseEvent) => void)
            | ((event: Event) => void),
        options?: AddEventListenerOptions,
    ): void {
        // hand the event to the listener
        this.#socket.addEventListener(
            type,
            (event: Event) => {
                Reflect.apply(listener, this, [event]);
            },
            options,
        );
    }
}

/** Networks. */
export const Network = {
    /** The system's network. */
    system: {
        fetch: (request, options) =>
            options?.certificate === undefined
                ? fetch(request)
                : fetchBun(request, options.certificate),
        connect: (url, protocols) =>
            new SystemSocket(
                new WebSocket(
                    url,
                    typeof protocols === "string" ? protocols : [...(protocols ?? [])],
                ),
            ),
        serve: serveBun,
    } satisfies Network,
};
