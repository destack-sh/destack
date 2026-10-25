import { NetworkError } from "./error.ts";
import type { ConnectOptions, SocketAddress, Socket } from "./socket/socket.ts";

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
    /** The Unix socket to send through in place of the URL's host, as a local listener's. */
    readonly unix?: string;
    /** Whether to decode a response's content encoding, true by default, as Bun's fetch has it. */
    readonly decompress?: boolean;
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

/** The network an environment reaches other origins through. */
export interface Network {
    /** Send a request and answer its response. */
    readonly fetch: (request: Request, options?: FetchOptions) => Promise<Response>;
    /** Open a connection to a host and port, such as an SMTP server's, or to a Unix socket. */
    readonly connect: (address: SocketAddress, options?: ConnectOptions) => Socket;
    /** Open a WebSocket to a URL, offering subprotocols. */
    readonly websocket: (url: string, protocols?: string | readonly string[]) => ClientSocket;
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
export const Network: { readonly system: Network } = {
    /** The web's network on every target: fetch and WebSockets, refusing what only hosts have. */
    system: {
        fetch: (request, options) => {
            // refuse a client certificate or a Unix socket, which only a host network sends with
            if (options?.certificate !== undefined || options?.unix !== undefined) {
                throw NetworkError.of(
                    "UNSUPPORTED",
                    "a client certificate or a Unix socket needs a host network",
                );
            }

            // refuse keeping a response's encoding, which the web's fetch always decodes
            if (options?.decompress === false) {
                throw NetworkError.of(
                    "UNSUPPORTED",
                    "keeping a response's encoding needs a host network",
                );
            }

            return fetch(request);
        },
        connect: () => {
            throw NetworkError.of("UNSUPPORTED", "a socket connection needs a host network");
        },
        websocket: (url, protocols) =>
            new SystemSocket(
                new WebSocket(
                    url,
                    typeof protocols === "string" ? protocols : [...(protocols ?? [])],
                ),
            ),
    },
};
