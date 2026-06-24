/** Binary transport for workspace protocol frames. */
export interface Transport {
    /** Send one binary frame. */
    send(bytes: Uint8Array): Promise<void>;
    /** Receive one binary frame. */
    receive(): Promise<Uint8Array>;
    /** Close the transport. */
    close(): void;
}

/** Error thrown by workspace protocol transports. */
export class TransportError extends Error {
    /** Create one transport error. */
    constructor(message: string) {
        super(message);
        this.name = "TransportError";
    }
}

/** WebSocket transport for browser and Node clients. */
export class WebSocketTransport implements Transport {
    readonly #socket: WebSocket;
    readonly #queue: Uint8Array[] = [];
    readonly #waiters: {
        readonly resolve: (bytes: Uint8Array) => void;
        readonly reject: (error: TransportError) => void;
    }[] = [];
    #closed: TransportError | undefined;

    /** Create one WebSocket transport from an open socket. */
    constructor(socket: WebSocket) {
        if (socket.readyState !== WebSocket.OPEN) {
            throw new TransportError("websocket transport requires an open socket");
        }

        this.#socket = socket;
        this.#socket.binaryType = "arraybuffer";
        this.#socket.addEventListener("message", (event) => {
            void this.#receiveSocketMessage(event.data).catch((error) => {
                this.#fail(transportError(error));
            });
        });
        this.#socket.addEventListener("close", () => {
            this.#fail(new TransportError("websocket transport closed"));
        });
        this.#socket.addEventListener("error", () => {
            this.#fail(new TransportError("websocket transport error"));
        });
    }

    /** Connect to one WebSocket endpoint. */
    static connect(url: string | URL): Promise<WebSocketTransport> {
        const socket = new WebSocket(url);
        socket.binaryType = "arraybuffer";

        return new Promise((resolve, reject) => {
            socket.addEventListener("open", () => {
                resolve(new WebSocketTransport(socket));
            }, { once: true });
            socket.addEventListener("error", () => {
                reject(new TransportError("websocket connection failed"));
            }, { once: true });
        });
    }

    /** Send one binary frame. */
    async send(bytes: Uint8Array): Promise<void> {
        if (this.#closed !== undefined) {
            throw this.#closed;
        }

        this.#socket.send(bytes);
    }

    /** Receive one binary frame. */
    receive(): Promise<Uint8Array> {
        if (this.#queue.length > 0) {
            return Promise.resolve(this.#queue.shift()!);
        }

        if (this.#closed !== undefined) {
            return Promise.reject(this.#closed);
        }

        return new Promise((resolve, reject) => {
            this.#waiters.push({ resolve, reject });
        });
    }

    /** Close the transport. */
    close(): void {
        this.#socket.close();
    }

    async #receiveSocketMessage(data: unknown): Promise<void> {
        const bytes = await messageBytes(data);
        const waiter = this.#waiters.shift();

        if (waiter !== undefined) {
            waiter.resolve(bytes);
        } else {
            this.#queue.push(bytes);
        }
    }

    #fail(error: TransportError): void {
        if (this.#closed !== undefined) {
            return;
        }

        this.#closed = error;
        while (this.#waiters.length > 0) {
            const waiter = this.#waiters.shift()!;
            waiter.reject(error);
        }
    }
}

async function messageBytes(data: unknown): Promise<Uint8Array> {
    if (data instanceof ArrayBuffer) {
        return new Uint8Array(data);
    }

    if (ArrayBuffer.isView(data)) {
        return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    }

    if (data instanceof Blob) {
        return new Uint8Array(await data.arrayBuffer());
    }

    throw new TransportError("websocket message was not binary");
}

function transportError(error: unknown): TransportError {
    if (error instanceof TransportError) {
        return error;
    }

    if (error instanceof Error) {
        return new TransportError(error.message);
    }

    return new TransportError(String(error));
}
