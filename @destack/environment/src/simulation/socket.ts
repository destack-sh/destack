import type { ClientSocket } from "../network/network.ts";

/** The close code of a connection lost without a close frame (RFC 6455). */
export const ABNORMAL_CLOSURE = 1006;

/** The close code of a normal close (RFC 6455). */
export const NORMAL_CLOSURE = 1000;

/** The end of a simulated connection that a client holds, as a browser's WebSocket. */
export class SimulatedWebSocket extends EventTarget implements ClientSocket {
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
    readyState: WebSocket["readyState"] = SimulatedWebSocket.CONNECTING;
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
        if (this.readyState === SimulatedWebSocket.CONNECTING) {
            throw new DOMException("the socket is still connecting", "InvalidStateError");
        } else if (this.readyState !== SimulatedWebSocket.OPEN) {
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
    close(code = NORMAL_CLOSURE, reason = ""): void {
        if (
            this.readyState === SimulatedWebSocket.OPEN ||
            this.readyState === SimulatedWebSocket.CONNECTING
        ) {
            this.readyState = SimulatedWebSocket.CLOSING;
            this.#close(code, reason);
        }
    }

    /** Open the socket with the server's chosen subprotocol. */
    opened(protocol: string): void {
        this.protocol = protocol;
        this.readyState = SimulatedWebSocket.OPEN;
        this.#dispatch(new Event("open"), this.onopen);
    }

    /** Take a message from the server's end. */
    received(message: string | Uint8Array<ArrayBuffer>): void {
        // drop messages after the close
        if (this.readyState !== SimulatedWebSocket.OPEN) {
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
        if (this.readyState === SimulatedWebSocket.CLOSED) {
            return;
        }
        this.readyState = SimulatedWebSocket.CLOSED;

        // report an abnormal close as a failure, then the close
        if (code === ABNORMAL_CLOSURE) {
            this.#dispatch(new Event("error"), this.onerror);
        }
        const event = new CloseEvent("close", {
            code,
            reason,
            wasClean: code !== ABNORMAL_CLOSURE,
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
