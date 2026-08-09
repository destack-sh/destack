/** Binary message transport for one RPC connection. */
export interface Transport {
    /** Send one complete binary message. */
    send(bytes: Uint8Array): Promise<void>;
    /** Receive one complete binary message. */
    receive(): Promise<Uint8Array>;
    /** Close the transport. */
    close(): void;
}

/** Error thrown by an RPC transport. */
export class TransportError extends Error {
    /** Create one transport error. */
    constructor(message: string) {
        super(message);
        this.name = "TransportError";
    }
}

/** WebSocket transport for browsers and compatible JavaScript runtimes. */
export class WebSocketTransport implements Transport {
    readonly #socket: WebSocket;
    readonly #messages: Uint8Array[] = [];
    readonly #receivers: Array<{
        readonly resolve: (bytes: Uint8Array) => void;
        readonly reject: (error: TransportError) => void;
    }> = [];
    #failure: TransportError | undefined;

    /** Create one transport from an open WebSocket. */
    constructor(socket: WebSocket) {
        if (socket.readyState !== WebSocket.OPEN) {
            throw new TransportError("RPC WebSocket must already be open");
        }

        this.#socket = socket;
        this.#socket.binaryType = "arraybuffer";
        this.#socket.addEventListener("message", (event) => {
            void this.#receiveSocketMessage(event.data).catch((error) => {
                this.#fail(asTransportError(error));
            });
        });
        this.#socket.addEventListener("close", () => {
            this.#fail(new TransportError("RPC WebSocket closed"));
        });
        this.#socket.addEventListener("error", () => {
            this.#fail(new TransportError("RPC WebSocket failed"));
        });
    }

    /** Connect to one RPC WebSocket endpoint. */
    static connect(url: string | URL): Promise<WebSocketTransport> {
        const socket = new WebSocket(url);
        socket.binaryType = "arraybuffer";

        return new Promise((resolve, reject) => {
            socket.addEventListener(
                "open",
                () => resolve(new WebSocketTransport(socket)),
                { once: true },
            );
            socket.addEventListener(
                "error",
                () => reject(new TransportError("RPC WebSocket connection failed")),
                { once: true },
            );
            socket.addEventListener(
                "close",
                () => reject(new TransportError("RPC WebSocket closed before opening")),
                { once: true },
            );
        });
    }

    /** Send one complete binary WebSocket message. */
    async send(bytes: Uint8Array): Promise<void> {
        if (this.#failure !== undefined) {
            throw this.#failure;
        }

        this.#socket.send(bytes);
    }

    /** Receive one complete binary WebSocket message. */
    receive(): Promise<Uint8Array> {
        const message = this.#messages.shift();
        if (message !== undefined) {
            return Promise.resolve(message);
        }
        if (this.#failure !== undefined) {
            return Promise.reject(this.#failure);
        }

        return new Promise((resolve, reject) => {
            this.#receivers.push({ resolve, reject });
        });
    }

    /** Close this WebSocket transport. */
    close(): void {
        this.#fail(new TransportError("RPC WebSocket closed"));
        this.#socket.close();
    }

    /** Convert and enqueue one WebSocket message. */
    async #receiveSocketMessage(value: unknown): Promise<void> {
        const bytes = await messageBytes(value);
        const receiver = this.#receivers.shift();

        if (receiver !== undefined) {
            receiver.resolve(bytes);
        } else {
            this.#messages.push(bytes);
        }
    }

    /** Permanently fail this transport and every pending receiver. */
    #fail(error: TransportError): void {
        if (this.#failure !== undefined) {
            return;
        }

        this.#failure = error;
        for (const receiver of this.#receivers.splice(0)) {
            receiver.reject(error);
        }
    }
}

/** Convert one WebSocket binary value into bytes. */
async function messageBytes(value: unknown): Promise<Uint8Array> {
    if (value instanceof ArrayBuffer) {
        return new Uint8Array(value);
    }
    if (ArrayBuffer.isView(value)) {
        return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    }
    if (value instanceof Blob) {
        return new Uint8Array(await value.arrayBuffer());
    }

    throw new TransportError("RPC WebSocket message was not binary");
}

/** Preserve transport errors while normalizing foreign failures. */
export function asTransportError(error: unknown): TransportError {
    if (error instanceof TransportError) {
        return error;
    }
    if (error instanceof Error) {
        return new TransportError(error.message);
    }

    return new TransportError(String(error));
}
