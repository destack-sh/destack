import type { ProtocolMessage } from "../_generated/protocol/envelope.js";
import { decodeProtocolMessage, encodeProtocolMessage } from "../_generated/protocol/envelope.js";
import { decodeValue, encodeValue } from "./serde.js";

const FRAME_MAGIC = [0x44, 0x53, 0x57, 0x50] as const;
const FRAME_VERSION = 1;
const FRAME_HEADER_SIZE = 12;

/** Error thrown while encoding or decoding workspace protocol frames. */
export class ProtocolCodecError extends Error {
    /** Create one protocol codec error. */
    constructor(message: string) {
        super(message);
        this.name = "ProtocolCodecError";
    }
}

/** Header fields for one workspace protocol frame. */
export type FrameHeader = {
    /** Magic bytes for protocol frames. */
    readonly magic: readonly [number, number, number, number];
    /** Frame format version. */
    readonly version: number;
    /** Reserved flags. */
    readonly flags: number;
    /** Payload length in bytes. */
    readonly payloadLength: number;
};

/** Encode one protocol message payload. */
export function encodeMessage(message: ProtocolMessage): Uint8Array {
    return encodeValue((writer) => {
        encodeProtocolMessage(writer, message);
    });
}

/** Decode one protocol message payload. */
export function decodeMessage(payload: Uint8Array | readonly number[]): ProtocolMessage {
    return decodeValue(payload, decodeProtocolMessage);
}

/** Encode one protocol message frame. */
export function encodeFrame(message: ProtocolMessage): Uint8Array {
    const payload = encodeMessage(message);

    return encodePayloadFrame(payload);
}

/** Decode one protocol message frame. */
export function decodeFrame(frame: Uint8Array | readonly number[]): ProtocolMessage {
    const payload = decodePayloadFrame(frame);

    return decodeMessage(payload);
}

/** Encode one protocol payload frame. */
export function encodePayloadFrame(payload: Uint8Array | readonly number[]): Uint8Array {
    const bytes = payload instanceof Uint8Array ? payload : new Uint8Array(payload);
    const header = encodeHeader(bytes.length);
    const frame = new Uint8Array(FRAME_HEADER_SIZE + bytes.length);

    frame.set(header, 0);
    frame.set(bytes, FRAME_HEADER_SIZE);

    return frame;
}

/** Decode one protocol payload frame. */
export function decodePayloadFrame(frame: Uint8Array | readonly number[]): Uint8Array {
    const bytes = frame instanceof Uint8Array ? frame : new Uint8Array(frame);
    const header = decodeHeader(bytes);
    const end = FRAME_HEADER_SIZE + header.payloadLength;

    if (bytes.length < end) {
        throw new ProtocolCodecError("unexpected end of frame payload");
    }

    return bytes.slice(FRAME_HEADER_SIZE, end);
}

/** Encode one frame header. */
export function encodeHeader(payloadLength: number): Uint8Array {
    if (!Number.isInteger(payloadLength) || payloadLength < 0 || payloadLength > 0xffffffff) {
        throw new ProtocolCodecError(`invalid payload length: ${payloadLength}`);
    }

    const bytes = new Uint8Array(FRAME_HEADER_SIZE);
    const view = new DataView(bytes.buffer);

    bytes.set(FRAME_MAGIC, 0);
    view.setUint16(4, FRAME_VERSION, true);
    view.setUint16(6, 0, true);
    view.setUint32(8, payloadLength, true);

    return bytes;
}

/** Decode one frame header. */
export function decodeHeader(bytes: Uint8Array): FrameHeader {
    if (bytes.length < FRAME_HEADER_SIZE) {
        throw new ProtocolCodecError("unexpected end of frame header");
    }

    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const magic = [bytes[0], bytes[1], bytes[2], bytes[3]] as const;
    const version = view.getUint16(4, true);
    const flags = view.getUint16(6, true);
    const payloadLength = view.getUint32(8, true);

    if (!magic.every((byte, index) => byte === FRAME_MAGIC[index])) {
        throw new ProtocolCodecError("invalid frame magic");
    }

    if (version !== FRAME_VERSION) {
        throw new ProtocolCodecError(`unsupported frame version: ${version}`);
    }

    return {
        magic,
        version,
        flags,
        payloadLength,
    };
}
