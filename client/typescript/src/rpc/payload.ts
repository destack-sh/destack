import type { Limits } from "../_generated/rpc/protocol/handshake.js";
import type { Message } from "../_generated/rpc/protocol/message.js";
import { Payload, type Payload as PayloadValue } from "../_generated/rpc/protocol/payload.js";
import { decodeMessage, encodeMessage } from "./codec.js";
import type { Transport } from "./transport.js";

const CHUNK_LENGTH_RESERVE = 16;
const MAX_SAFE_BIGINT = BigInt(Number.MAX_SAFE_INTEGER);

/** Serialized message sender with transparent deferred payload chunking. */
export class MessageSender {
    readonly #transport: Transport;
    readonly #maxMessageBytes: number;
    readonly #maxPayloadBytes: number;
    #nextPayload = 1n;
    #sending = Promise.resolve();
    #isFailed = false;
    #failure: unknown;

    /** Create one sender for negotiated connection limits. */
    constructor(transport: Transport, limits: Limits) {
        this.#transport = transport;
        this.#maxMessageBytes = safeLength(limits.maxMessageBytes, "message limit");
        this.#maxPayloadBytes = safeLength(limits.maxPayloadBytes, "payload limit");
    }

    /** Send one complete message and its contiguous payload chunks. */
    send(message: Message): Promise<void> {
        const sending = this.#sending.then(async () => {
            if (this.#isFailed) {
                throw this.#failure;
            }

            await this.#send(message);
        });
        this.#sending = sending.catch((error) => {
            this.#isFailed = true;
            this.#failure = error;
        });

        return sending;
    }

    /** Close the underlying transport. */
    close(): void {
        this.#transport.close();
    }

    /** Encode one message inline or defer its one value payload. */
    async #send(message: Message): Promise<void> {
        const payload = messagePayload(message);
        if (payload?.kind === "inline" && payload.inline.length > this.#maxPayloadBytes) {
            throw new RpcProtocolError(
                `RPC payload exceeds ${this.#maxPayloadBytes} bytes: ${payload.inline.length}`,
            );
        }

        const bytes = encodeMessage(message);
        if (bytes.length <= this.#maxMessageBytes) {
            await this.#transport.send(bytes);

            return;
        }
        if (payload?.kind !== "inline") {
            throw new RpcProtocolError("oversized RPC message has no inline payload");
        }

        const id = this.#nextPayload;
        this.#nextPayload += 1n;
        const deferred = replaceMessagePayload(message, Payload.deferred(id, BigInt(payload.inline.length)));
        const deferredBytes = encodeMessage(deferred);
        if (deferredBytes.length > this.#maxMessageBytes) {
            throw new RpcProtocolError(
                `deferred RPC message exceeds ${this.#maxMessageBytes} bytes: ${deferredBytes.length}`,
            );
        }

        await this.#transport.send(deferredBytes);
        await this.#sendChunks(id, new Uint8Array(payload.inline));
    }

    /** Send contiguous chunks under this sender's serialization order. */
    async #sendChunks(payload: bigint, bytes: Uint8Array): Promise<void> {
        const empty: Message = {
            kind: "chunk",
            chunk: { payload, bytes: new Uint8Array() },
        };
        const overhead = encodeMessage(empty).length;
        const chunkBytes = this.#maxMessageBytes - overhead - CHUNK_LENGTH_RESERVE;
        if (chunkBytes <= 0) {
            throw new RpcProtocolError("RPC message limit cannot hold a deferred payload chunk");
        }

        for (let offset = 0; offset < bytes.length; offset += chunkBytes) {
            const chunk: Message = {
                kind: "chunk",
                chunk: {
                    payload,
                    bytes: bytes.slice(offset, offset + chunkBytes),
                },
            };
            await this.#transport.send(encodeMessage(chunk));
        }
    }
}

/** Message receiver with strict contiguous deferred payload assembly. */
export class MessageReceiver {
    readonly #transport: Transport;
    readonly #maxMessageBytes: number;
    readonly #maxPayloadBytes: number;
    #pending: PendingPayload | undefined;

    /** Create one receiver for negotiated connection limits. */
    constructor(transport: Transport, limits: Limits) {
        this.#transport = transport;
        this.#maxMessageBytes = safeLength(limits.maxMessageBytes, "message limit");
        this.#maxPayloadBytes = safeLength(limits.maxPayloadBytes, "payload limit");
    }

    /** Receive one complete RPC message. */
    async receive(): Promise<Message> {
        while (true) {
            const bytes = await this.#transport.receive();
            if (bytes.length > this.#maxMessageBytes) {
                throw new RpcProtocolError(
                    `RPC message exceeds ${this.#maxMessageBytes} bytes: ${bytes.length}`,
                );
            }
            const message = decodeMessage(bytes);

            // assemble only the contiguous chunks declared by the pending message
            if (this.#pending !== undefined) {
                if (message.kind !== "chunk") {
                    throw new RpcProtocolError("deferred RPC payload chunks are not contiguous");
                }
                const completed = this.#push(message.chunk.payload, message.chunk.bytes);
                if (completed !== undefined) {
                    return completed;
                }

                continue;
            }
            if (message.kind === "chunk") {
                throw new RpcProtocolError("RPC payload chunk has no deferred message");
            }

            const payload = messagePayload(message);
            if (payload === undefined) {
                return message;
            }
            if (payload.kind === "inline") {
                this.#validatePayload(payload.inline.length);

                return message;
            }

            const byteLen = safeLength(payload.byteLen, "deferred payload length");
            this.#validatePayload(byteLen);
            if (byteLen === 0) {
                return replaceMessagePayload(message, Payload.inline(new Uint8Array()));
            }

            this.#pending = {
                message,
                payload: payload.id,
                byteLen,
                bytes: new Uint8Array(byteLen),
                received: 0,
            };
        }
    }

    /** Append one exact payload chunk and return its completed message. */
    #push(payload: bigint, bytes: Uint8Array | readonly number[]): Message | undefined {
        const pending = this.#pending;
        if (pending === undefined) {
            throw new RpcProtocolError("RPC payload chunk has no deferred message");
        }
        if (payload !== pending.payload) {
            throw new RpcProtocolError("RPC payload chunk identity does not match its message");
        }

        const received = pending.received + bytes.length;
        if (received > pending.byteLen) {
            throw new RpcProtocolError("RPC payload chunks exceed their declared length");
        }
        pending.bytes.set(bytes, pending.received);
        pending.received = received;
        if (received < pending.byteLen) {
            return undefined;
        }

        this.#pending = undefined;

        return replaceMessagePayload(pending.message, Payload.inline(pending.bytes));
    }

    /** Enforce the negotiated assembled payload limit. */
    #validatePayload(actual: number): void {
        if (actual > this.#maxPayloadBytes) {
            throw new RpcProtocolError(
                `RPC payload exceeds ${this.#maxPayloadBytes} bytes: ${actual}`,
            );
        }
    }
}

/** Protocol violation detected by the TypeScript RPC runtime. */
export class RpcProtocolError extends Error {
    /** Create one protocol error. */
    constructor(message: string) {
        super(message);
        this.name = "RpcProtocolError";
    }
}

/** One message awaiting its declared payload bytes. */
type PendingPayload = {
    /** Owning message. */
    readonly message: Message;
    /** Expected payload identifier. */
    readonly payload: bigint;
    /** Exact declared byte length. */
    readonly byteLen: number;
    /** Bounded assembled byte storage. */
    readonly bytes: Uint8Array;
    /** Bytes assembled so far. */
    received: number;
};

/** Return the one value payload carried by a message. */
function messagePayload(message: Message): PayloadValue | undefined {
    switch (message.kind) {
        case "start":
            return message.start.request.value;
        case "item":
            return message.item.payload;
        case "complete":
            return message.complete.kind === "response" ? message.complete.response.value : undefined;
        case "close":
        case "window":
        case "cancel":
        case "chunk":
            return undefined;
    }
}

/** Replace the one value payload carried by a message. */
function replaceMessagePayload(message: Message, payload: PayloadValue): Message {
    switch (message.kind) {
        case "start":
            return {
                kind: "start",
                start: {
                    ...message.start,
                    request: { ...message.start.request, value: payload },
                },
            };
        case "item":
            return { kind: "item", item: { ...message.item, payload } };
        case "complete":
            if (message.complete.kind === "response") {
                return {
                    kind: "complete",
                    complete: {
                        ...message.complete,
                        response: { ...message.complete.response, value: payload },
                    },
                };
            }

            break;
        case "close":
        case "window":
        case "cancel":
        case "chunk":
            break;
    }

    throw new RpcProtocolError("RPC message has no replaceable payload");
}

/** Convert one bounded wire length into a JavaScript number. */
export function safeLength(value: bigint, name: string): number {
    if (value > MAX_SAFE_BIGINT) {
        throw new RpcProtocolError(`${name} does not fit JavaScript's exact integer range`);
    }

    return Number(value);
}
