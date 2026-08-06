import { asTransportError, TransportError, type Transport } from "./transport.js";

/** Native in-process RPC session consumed by an embedded transport. */
export interface EmbeddedSession {
    /** Dispatch one complete inbound RPC message. */
    dispatch(bytes: Uint8Array): readonly unknown[];
    /** Poll cooperatively ready RPC calls. */
    poll(): readonly unknown[];
    /** Return whether a cooperative RPC call requested another poll. */
    isReady(): boolean;
    /** Register a host callback for readiness changes when the embedding supports it. */
    onReady?(wake: () => void): void;
    /** Close this session. */
    close(): void;
}

/** Message transport backed by one in-process RPC session. */
export class EmbeddedTransport implements Transport {
    readonly #session: EmbeddedSession;
    readonly #messages: Uint8Array[] = [];
    readonly #receivers: Array<{
        readonly resolve: (bytes: Uint8Array) => void;
        readonly reject: (error: TransportError) => void;
    }> = [];
    #failure: TransportError | undefined;
    #isPollScheduled = false;

    /** Create one transport over an in-process session. */
    constructor(session: EmbeddedSession) {
        this.#session = session;
        this.#session.onReady?.(() => this.#schedulePoll());
    }

    /** Dispatch one complete inbound message. */
    async send(bytes: Uint8Array): Promise<void> {
        if (this.#failure !== undefined) {
            throw this.#failure;
        }

        try {
            const messages = this.#session.dispatch(bytes);
            this.#enqueueAll(messages);
            this.#schedulePoll();
        } catch (error) {
            const failure = asTransportError(error);
            this.#fail(failure);

            throw failure;
        }
    }

    /** Receive one complete outbound message. */
    receive(): Promise<Uint8Array> {
        this.#poll();

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

    /** Poll the embedded session once. */
    #poll(): void {
        if (this.#failure !== undefined) {
            return;
        }

        try {
            const messages = this.#session.poll();
            this.#enqueueAll(messages);
            this.#schedulePoll();
        } catch (error) {
            this.#fail(asTransportError(error));
        }
    }

    /** Schedule another cooperative poll after yielding to the JavaScript host. */
    #schedulePoll(): void {
        if (this.#isPollScheduled || this.#failure !== undefined) {
            return;
        }

        try {
            if (!this.#session.isReady()) {
                return;
            }
        } catch (error) {
            this.#fail(asTransportError(error));

            return;
        }

        this.#isPollScheduled = true;
        setTimeout(() => {
            this.#isPollScheduled = false;
            this.#poll();
        }, 0);
    }

    /** Convert and enqueue complete outbound messages. */
    #enqueueAll(messages: readonly unknown[]): void {
        for (const message of messages) {
            this.#enqueue(messageBytes(message));
        }
    }

    /** Close this transport and its native session. */
    close(): void {
        if (this.#failure !== undefined) {
            return;
        }

        this.#session.close();
        this.#fail(new TransportError("embedded RPC session closed"));
    }

    /** Enqueue one complete outbound message. */
    #enqueue(bytes: Uint8Array): void {
        const receiver = this.#receivers.shift();

        if (receiver === undefined) {
            this.#messages.push(bytes);
        } else {
            receiver.resolve(bytes);
        }
    }

    /** Permanently fail every pending receiver. */
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

/** Convert one native binary result into bytes. */
function messageBytes(value: unknown): Uint8Array {
    if (value instanceof Uint8Array) {
        return value;
    }
    if (value instanceof ArrayBuffer) {
        return new Uint8Array(value);
    }
    if (ArrayBuffer.isView(value)) {
        return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    }
    if (Array.isArray(value)) {
        return new Uint8Array(value);
    }

    throw new TransportError("embedded RPC message was not binary");
}
