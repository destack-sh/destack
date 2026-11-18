import { WebSocketClosure } from "@destack/environment";
import { TunnelError } from "../error/index.ts";

/** The most bytes one WebSocket message carries through a tunnel: 1 MiB, the hosts' own limit. */
const MAX_MESSAGE_BYTES = 1024 * 1024;

/** The FIN bit of a frame's first byte: the frame is a whole message (RFC 6455 5.2). */
const FIN_BIT = 0x80;

/** The payload length that a 16-bit length follows (RFC 6455 5.2). */
const LENGTH_16 = 126;

/** The payload length that a 64-bit length follows (RFC 6455 5.2). */
const LENGTH_64 = 127;

/** The opcodes of the WebSocket frames a tunnel stream carries (RFC 6455 5.2). */
export const WebSocketOpcode = {
    /** A text message. */
    text: 0x1,
    /** A binary message. */
    binary: 0x2,
    /** A close, with its code and reason. */
    close: 0x8,
} as const;

/** One of the opcodes. */
export type WebSocketOpcode = (typeof WebSocketOpcode)[keyof typeof WebSocketOpcode];

/** The end of a WebSocket that messages go to: a client's socket or a host's. */
export interface WebSocketEnd {
    /** Send one message, settling once the socket took it, which holds back the next frame. */
    readonly send: (message: string | Uint8Array<ArrayBuffer>) => Promise<void>;
    /** Close with a code and reason. */
    readonly close: (code?: number, reason?: string) => void;
}

/** One WebSocket frame on a tunnel stream, unmasked as RFC 8441 carries them. */
export class WebSocketFrame {
    /** What the frame carries. */
    readonly opcode: WebSocketOpcode;
    /** The message's bytes, or the close's code and reason. */
    readonly payload: Uint8Array<ArrayBuffer>;

    /** Describe a frame. */
    constructor(opcode: WebSocketOpcode, payload: Uint8Array<ArrayBuffer>) {
        this.opcode = opcode;
        this.payload = payload;
    }

    /** Frame a text or binary message. */
    static message(message: string | Uint8Array<ArrayBuffer>): WebSocketFrame {
        return typeof message === "string"
            ? new WebSocketFrame(WebSocketOpcode.text, new TextEncoder().encode(message))
            : new WebSocketFrame(WebSocketOpcode.binary, message);
    }

    /** Frame a close with its code and reason, a code no peer sends as going away. */
    static close(code: number, reason: string): WebSocketFrame {
        // write the code, then the reason's text
        const text = new TextEncoder().encode(reason);
        const payload = new Uint8Array(2 + text.length);
        new DataView(payload.buffer).setUint16(0, WebSocketClosure.sendable(code));
        payload.set(text, 2);

        return new WebSocketFrame(WebSocketOpcode.close, payload);
    }

    /** Write the frame with its header: FIN, opcode and payload length, unmasked. */
    encode(): Uint8Array<ArrayBuffer> {
        // take the shortest length form that fits the payload
        const { length } = this.payload;
        const lengthBytes = length < LENGTH_16 ? 0 : length <= 0xffff ? 2 : 8;
        const encoded = new Uint8Array(2 + lengthBytes + length);
        const view = new DataView(encoded.buffer);
        encoded[0] = FIN_BIT | this.opcode;

        // write the length in its form, then the payload
        if (lengthBytes === 0) {
            encoded[1] = length;
        } else if (lengthBytes === 2) {
            encoded[1] = LENGTH_16;
            view.setUint16(2, length);
        } else {
            encoded[1] = LENGTH_64;
            view.setBigUint64(2, BigInt(length));
        }
        encoded.set(this.payload, 2 + lengthBytes);

        return encoded;
    }

    /** Read the frames a stream's bytes carry, in order, as they arrive. */
    static async *read(readable: ReadableStream<Uint8Array>): AsyncGenerator<WebSocketFrame> {
        let buffered = new Uint8Array(0);
        for await (const chunk of readable) {
            // append the chunk, then take every whole frame at the front
            const joined = new Uint8Array(buffered.length + chunk.length);
            joined.set(buffered);
            joined.set(chunk, buffered.length);
            buffered = joined;
            let parsed = WebSocketFrame.#parse(buffered);
            while (parsed !== undefined) {
                yield parsed.frame;
                buffered = buffered.subarray(parsed.length);
                parsed = WebSocketFrame.#parse(buffered);
            }
        }

        // refuse a stream that ends inside a frame
        if (buffered.length > 0) {
            throw TunnelError.of(
                "PROTOCOL_VIOLATION_INVALID",
                "websocket frame is invalid: the stream ended inside it",
            );
        }
    }

    /** Hand a stream's frames to a socket's end in order, closing it as the stream ends. */
    static async deliver(readable: ReadableStream<Uint8Array>, end: WebSocketEnd): Promise<void> {
        let isClosed = false;
        try {
            for await (const frame of WebSocketFrame.read(readable)) {
                // close with the frame's code and reason
                if (frame.opcode === WebSocketOpcode.close) {
                    const { code, reason } = frame.closing();
                    isClosed = true;
                    end.close(code, reason);

                    return;
                }

                // send the message on, reading the next frame once the socket took it
                await end.send(frame.message());
            }
        } catch (error) {
            // end on the stream's own failure, as its tunnel went away, and rethrow a broken frame
            if (TunnelError.is(error, "PROTOCOL_VIOLATION_INVALID")) {
                throw error;
            }
        } finally {
            // close a socket whose stream ended or failed without a close, as its tunnel went away
            if (!isClosed) {
                end.close(WebSocketClosure.goingAway, "the tunnel stream ended");
            }
        }
    }

    /** Read a text or binary frame's message. */
    message(): string | Uint8Array<ArrayBuffer> {
        return this.opcode === WebSocketOpcode.text
            ? new TextDecoder().decode(this.payload)
            : this.payload;
    }

    /** Read a close frame's code and reason. */
    closing(): { readonly code: number; readonly reason: string } {
        const view = new DataView(this.payload.buffer, this.payload.byteOffset);

        return {
            code: view.getUint16(0),
            reason: new TextDecoder().decode(this.payload.subarray(2)),
        };
    }

    /** Read the frame at the front of the bytes and its length, none until it arrived whole. */
    static #parse(
        bytes: Uint8Array,
    ): { readonly frame: WebSocketFrame; readonly length: number } | undefined {
        // wait for the header and the length form it names
        if (bytes.length < 2) {
            return undefined;
        }
        const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        const [first = 0, second = 0] = bytes;
        const marker = second & 0x7f;
        const lengthBytes = marker === LENGTH_16 ? 2 : marker === LENGTH_64 ? 8 : 0;
        if (bytes.length < 2 + lengthBytes) {
            return undefined;
        }

        // read the opcode and length, refusing a fragment, a mask, an unknown opcode or an oversized message
        const opcode = first & 0x0f;
        const length =
            lengthBytes === 0
                ? marker
                : lengthBytes === 2
                  ? view.getUint16(2)
                  : Number(view.getBigUint64(2));
        if ((first & FIN_BIT) === 0 || (second & 0x80) !== 0 || !isOpcode(opcode)) {
            throw TunnelError.of(
                "PROTOCOL_VIOLATION_INVALID",
                "websocket frame is invalid: a tunnel carries whole, unmasked text, binary or close frames",
            );
        } else if (length > MAX_MESSAGE_BYTES) {
            throw TunnelError.of(
                "PROTOCOL_VIOLATION_INVALID",
                `websocket frame is invalid: it is longer than ${MAX_MESSAGE_BYTES} bytes`,
            );
        }

        // wait for the whole payload
        const start = 2 + lengthBytes;
        if (bytes.length < start + length) {
            return undefined;
        }
        const payload = bytes.slice(start, start + length);

        return { frame: new WebSocketFrame(opcode, payload), length: start + length };
    }
}

/** Report whether a header's opcode is one a tunnel stream carries. */
function isOpcode(opcode: number): opcode is WebSocketOpcode {
    return (
        opcode === WebSocketOpcode.text ||
        opcode === WebSocketOpcode.binary ||
        opcode === WebSocketOpcode.close
    );
}
