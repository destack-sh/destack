import type { ClientSocket } from "./network.ts";

/** The prefix of the subprotocol with a bearer token in base64url, as Kubernetes offers its own. */
const BEARER_PROTOCOL_PREFIX = "destack.bearer.";

/** The close codes a peer reports but never sends (RFC 6455 7.4.1). */
const UNSENT_CODES: ReadonlySet<number> = new Set([1005, 1006, 1015]);

/** The headers a WebSocket client writes itself for each upgrade (RFC 6455 4.1). */
const CLIENT_HEADERS = [
    "upgrade",
    "connection",
    "sec-websocket-key",
    "sec-websocket-version",
    "sec-websocket-extensions",
    "sec-websocket-protocol",
];

/** The close codes of a WebSocket (RFC 6455 7.4.1). */
export const WebSocketClosure = {
    /** A normal close. */
    normal: 1000,
    /** An end that goes away, such as a server stopping or a page leaving. */
    goingAway: 1001,
    /** A connection lost without a close frame, which a peer reports but never sends. */
    abnormal: 1006,
    /** A message or opening that breaks the endpoint's policy, such as a refused upgrade. */
    policy: 1008,
    /** A server restarting, which its client reconnects after. */
    restart: 1012,

    /** Read a close code a peer may send, a reported-only one as going away. */
    sendable(code: number): number {
        return UNSENT_CODES.has(code) ? WebSocketClosure.goingAway : code;
    },
};

/** A received WebSocket message as text, or as the bytes of its ArrayBuffer. */
export const WebSocketMessage = {
    /** Read a message event's data, which a client socket receives as text or an ArrayBuffer. */
    of(data: unknown): string | Uint8Array<ArrayBuffer> {
        // take text as it is
        if (typeof data === "string") {
            return data;
        }
        // take the bytes of a binary message
        else if (data instanceof ArrayBuffer) {
            return new Uint8Array(data);
        }

        throw new TypeError("a client socket receives text or ArrayBuffers");
    },
};

/** The bearer tokens clients offer as subprotocols, as browsers set no header on an opening. */
export const BearerProtocol = {
    /** Write a bearer token as the subprotocol a client offers in its place. */
    write(token: string): string {
        const encoded = new TextEncoder()
            .encode(token)
            .toBase64({ alphabet: "base64url", omitPadding: true });

        return `${BEARER_PROTOCOL_PREFIX}${encoded}`;
    },

    /** Read the one bearer token the headers' subprotocols offer, absent for none or several. */
    read(headers: Headers): string | undefined {
        // require one bearer subprotocol
        const bearers = offered(headers).filter((protocol) =>
            protocol.startsWith(BEARER_PROTOCOL_PREFIX),
        );
        const [bearer] = bearers;
        if (bearers.length !== 1 || bearer === undefined) {
            return undefined;
        }

        // decode its token, absent for invalid base64url
        try {
            const encoded = bearer.slice(BEARER_PROTOCOL_PREFIX.length);
            const bytes = Uint8Array.fromBase64(encoded, { alphabet: "base64url" });

            return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
        } catch {
            return undefined;
        }
    },
};

/** A request's WebSocket upgrade (RFC 6455 4.1): the request and the subprotocols it offers. */
export class WebSocketUpgrade {
    /** The upgrade's request. */
    readonly request: Request;
    /** The subprotocols the client offers, in its order. */
    readonly protocols: readonly string[];

    /** Take an upgrade's request and its offered subprotocols. */
    private constructor(request: Request, protocols: readonly string[]) {
        this.request = request;
        this.protocols = protocols;
    }

    /** Read a request's WebSocket upgrade, absent for a plain request. */
    static of(request: Request): WebSocketUpgrade | undefined {
        // take a request asking to switch to a WebSocket
        if (request.headers.get("upgrade")?.toLowerCase() !== "websocket") {
            return undefined;
        }

        return new WebSocketUpgrade(request, offered(request.headers));
    }

    /** Describe the upgrade of a socket at a URL offering subprotocols, beside other headers. */
    static to(
        url: string,
        protocols: readonly string[],
        headers: Headers = new Headers(),
    ): WebSocketUpgrade {
        // ask to switch at the URL's HTTP form, offering the subprotocols
        const upgraded = new Headers(headers);
        upgraded.set("upgrade", "websocket");
        upgraded.set("connection", "Upgrade");
        if (protocols.length > 0) {
            upgraded.set("sec-websocket-protocol", protocols.join(", "));
        }
        const request = new Request(url.replace(/^ws/u, "http"), { headers: upgraded });

        return new WebSocketUpgrade(request, protocols);
    }

    /** The upgrade's end-to-end headers, without those a client writes itself for each upgrade. */
    forwarded(): Headers {
        const headers = new Headers(this.request.headers);
        for (const name of CLIENT_HEADERS) {
            headers.delete(name);
        }

        return headers;
    }
}

/** The client end of an in-memory connection, as a browser's WebSocket holds it. */
export class MemoryWebSocket extends EventTarget implements ClientSocket {
    /** The state before the connection opens. */
    static readonly CONNECTING = 0;
    /** The state while the connection is open. */
    static readonly OPEN = 1;
    /** The state while the connection closes. */
    static readonly CLOSING = 2;
    /** The state once the connection closed. */
    static readonly CLOSED = 3;
    /** The state before the connection opens. */
    readonly CONNECTING = 0;
    /** The state while the connection is open. */
    readonly OPEN = 1;
    /** The state while the connection closes. */
    readonly CLOSING = 2;
    /** The state once the connection closed. */
    readonly CLOSED = 3;

    /** The URL the socket connects to. */
    readonly url: string;
    /** The extensions the server chose, none here. */
    readonly extensions = "";
    /** The bytes queued but not yet sent, none here since sends queue on the network. */
    readonly bufferedAmount = 0;
    /** The form binary messages arrive in: ArrayBuffers, as every client socket, or Blobs. */
    binaryType = "arraybuffer";
    /** The subprotocol the server chose. */
    protocol = "";
    /** The connection's state. */
    readyState: WebSocket["readyState"] = MemoryWebSocket.CONNECTING;
    /** Take the open. */
    onopen: ((event: Event) => unknown) | null = null;
    /** Take a message. */
    onmessage: ((event: MessageEvent) => unknown) | null = null;
    /** Take the close. */
    onclose: ((event: CloseEvent) => unknown) | null = null;
    /** Take a failure. */
    onerror: ((event: Event) => unknown) | null = null;
    /** Send a message to the server's end. */
    readonly #send: (message: string | Uint8Array<ArrayBuffer>) => void;
    /** Close the connection from this end. */
    readonly #close: (code: number, reason: string) => void;

    /** Create a client's end of a connection over a network's send and close. */
    constructor(
        url: string,
        send: (message: string | Uint8Array<ArrayBuffer>) => void,
        close: (code: number, reason: string) => void,
    ) {
        // connect the socket's sends and close to the network
        super();
        this.url = url;
        this.#send = send;
        this.#close = close;
    }

    /** Take the open or a failure. */
    override addEventListener(
        type: "open" | "error",
        listener: (event: Event) => void,
        options?: AddEventListenerOptions,
    ): void;
    /** Take a message. */
    override addEventListener(
        type: "message",
        listener: (event: MessageEvent) => void,
        options?: AddEventListenerOptions,
    ): void;
    /** Take the close. */
    override addEventListener(
        type: "close",
        listener: (event: CloseEvent) => void,
        options?: AddEventListenerOptions,
    ): void;
    /** Take an event. */
    override addEventListener(
        type: string,
        listener:
            | ((event: MessageEvent) => void)
            | ((event: CloseEvent) => void)
            | ((event: Event) => void),
        options?: AddEventListenerOptions,
    ): void {
        // hand the event to the listener
        super.addEventListener(
            type,
            (event: Event) => {
                Reflect.apply(listener, this, [event]);
            },
            options,
        );
    }

    /** Send a message, refusing one before the connection opens. */
    send(message: string | ArrayBufferLike | Blob | ArrayBufferView): void {
        // refuse a socket that is still connecting, and drop sends after the close
        if (this.readyState === MemoryWebSocket.CONNECTING) {
            throw new DOMException("the socket is still connecting", "InvalidStateError");
        } else if (this.readyState !== MemoryWebSocket.OPEN) {
            return;
        }

        // copy the message as text or bytes
        if (typeof message === "string") {
            this.#send(message);
        } else if (message instanceof Blob) {
            throw new TypeError("a simulated socket can only send text or bytes");
        } else if (ArrayBuffer.isView(message)) {
            this.#send(
                new Uint8Array(message.buffer, message.byteOffset, message.byteLength).slice(),
            );
        } else {
            this.#send(new Uint8Array(message).slice());
        }
    }

    /** Close the connection. */
    close(code = WebSocketClosure.normal, reason = ""): void {
        if (
            this.readyState === MemoryWebSocket.OPEN ||
            this.readyState === MemoryWebSocket.CONNECTING
        ) {
            this.readyState = MemoryWebSocket.CLOSING;
            this.#close(code, reason);
        }
    }

    /** Open the socket with the server's chosen subprotocol. */
    opened(protocol: string): void {
        this.protocol = protocol;
        this.readyState = MemoryWebSocket.OPEN;
        this.#dispatch(new Event("open"), this.onopen);
    }

    /** Take a message from the server's end. */
    received(message: string | Uint8Array<ArrayBuffer>): void {
        // drop messages after the close
        if (this.readyState !== MemoryWebSocket.OPEN) {
            return;
        }

        // hand text over as is, and bytes in the socket's binary form
        const data =
            typeof message === "string"
                ? message
                : this.binaryType === "arraybuffer"
                  ? message.buffer
                  : new Blob([message]);
        this.#dispatch(new MessageEvent("message", { data }), this.onmessage);
    }

    /** End the socket, failing first for an abnormal close. */
    closed(code: number, reason: string): void {
        // end once
        if (this.readyState === MemoryWebSocket.CLOSED) {
            return;
        }
        this.readyState = MemoryWebSocket.CLOSED;

        // report an abnormal close as a failure, then the close
        if (code === WebSocketClosure.abnormal) {
            this.#dispatch(new Event("error"), this.onerror);
        }
        const event = new CloseEvent("close", {
            code,
            reason,
            wasClean: code !== WebSocketClosure.abnormal,
        });
        this.#dispatch(event, this.onclose);
    }

    /** Dispatch an event to its listeners and its handler property. */
    #dispatch<Fired extends Event>(
        event: Fired,
        handler: ((event: Fired) => unknown) | null,
    ): void {
        handler?.(event);
        this.dispatchEvent(event);
    }
}

/** Read the subprotocols headers offer, in their order. */
function offered(headers: Headers): string[] {
    return (headers.get("sec-websocket-protocol") ?? "")
        .split(",")
        .map((protocol) => protocol.trim())
        .filter((protocol) => protocol.length > 0);
}
