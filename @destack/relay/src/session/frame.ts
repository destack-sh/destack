/** The length of a frame header: type, flags, stream and value. */
const HEADER_BYTES = 10;

/** The most bytes one frame carries, a request head included: 64 KiB, as HTTP servers cap heads at 8 to 64 KiB. */
export const MAX_PAYLOAD_BYTES = 64 * 1024;

/** The most bytes one frame's message takes. */
export const MAX_FRAME_BYTES = HEADER_BYTES + MAX_PAYLOAD_BYTES;

/** What a frame carries, as yamux calls its frame types. */
export const FrameType = {
    /** Bytes of a stream; a SYN data frame opens the stream and carries its header instead. */
    data: 0,
    /** Credit for more bytes of a stream, the value in bytes. */
    window: 1,
    /** A liveness probe on the session, answered with ACK, the value opaque. */
    ping: 2,
    /** The end of the session, the value its code. */
    goAway: 3,
} as const;

/** One of the frame types. */
export type FrameType = (typeof FrameType)[keyof typeof FrameType];

/** The flags of a frame, as yamux calls them. */
export const FrameFlag = {
    /** Open a stream. */
    syn: 1,
    /** Answer a ping. */
    ack: 2,
    /** End the sender's half of a stream. */
    fin: 4,
    /** Abort a stream. */
    rst: 8,
} as const;

/** The codes a session ends with. */
export const GoAwayCode = {
    /** The session ends normally. */
    normal: 0,
    /** The peer broke the protocol. */
    protocol: 1,
} as const;

/** One yamux-shaped frame of a session, sent as one binary WebSocket message. */
export class Frame {
    /** What the frame carries. */
    readonly type: FrameType;
    /** The frame's flags, a bit set of `FrameFlag`. */
    readonly flags: number;
    /** The stream the frame belongs to, zero for the session. */
    readonly stream: number;
    /** The window delta, ping value or go-away code. */
    readonly value: number;
    /** The stream bytes of a data frame, or the stream header of a SYN. */
    readonly payload: Uint8Array;

    /** Describe a frame. */
    constructor(
        type: FrameType,
        flags: number,
        stream: number,
        value = 0,
        payload: Uint8Array = new Uint8Array(0),
    ) {
        // keep the header's fields and the payload
        this.type = type;
        this.flags = flags;
        this.stream = stream;
        this.value = value;
        this.payload = payload;
    }

    /** Read a frame from a message and refuse one shorter than a header, longer than a frame, or of an unknown type. */
    static decode(message: Uint8Array): Frame {
        // require a whole header and a bounded payload
        if (message.length < HEADER_BYTES) {
            throw new TypeError("frame is shorter than its header");
        } else if (message.length > MAX_FRAME_BYTES) {
            throw new TypeError(`frame is longer than ${MAX_FRAME_BYTES} bytes`);
        }
        const view = new DataView(message.buffer, message.byteOffset, message.byteLength);
        const type = view.getUint8(0);
        if (!isFrameType(type)) {
            throw new TypeError(`frame type ${type} is unknown`);
        }

        return new Frame(
            type,
            view.getUint8(1),
            view.getUint32(2),
            view.getUint32(6),
            message.subarray(HEADER_BYTES),
        );
    }

    /** Whether the frame carries a flag. */
    has(flag: number): boolean {
        return (this.flags & flag) !== 0;
    }

    /** Write the frame as one message. */
    encode(): Uint8Array<ArrayBuffer> {
        // write the header's fields, then the payload
        const message = new Uint8Array(HEADER_BYTES + this.payload.length);
        const view = new DataView(message.buffer);
        message[0] = this.type;
        message[1] = this.flags;
        view.setUint32(2, this.stream);
        view.setUint32(6, this.value);
        message.set(this.payload, HEADER_BYTES);

        return message;
    }
}

/** Report whether a header byte is one of the frame types. */
function isFrameType(type: number): type is FrameType {
    return type <= FrameType.goAway;
}
