import { expect, refusal, test } from "@destack/test";
import { WebSocketFrame, WebSocketOpcode } from "./websocket.ts";

/** The payload lengths at each edge of the three length forms (RFC 6455 5.2). */
const LENGTHS = [0, 125, 126, 65_535, 65_536];

/** Stream bytes in chunks of a size, as a tunnel stream hands them over. */
function chunked(bytes: Uint8Array, size: number): ReadableStream<Uint8Array> {
    let offset = 0;

    return new ReadableStream({
        pull(controller) {
            // hand the next chunk, then end
            if (offset >= bytes.length) {
                controller.close();

                return;
            }
            controller.enqueue(bytes.slice(offset, offset + size));
            offset += size;
        },
    });
}

/** Join frames' encodings into one byte run. */
function joined(frames: readonly WebSocketFrame[]): Uint8Array {
    const encoded = frames.map((frame) => frame.encode());
    const bytes = new Uint8Array(encoded.reduce((sum, each) => sum + each.length, 0));
    let offset = 0;
    for (const each of encoded) {
        bytes.set(each, offset);
        offset += each.length;
    }

    return bytes;
}

/** Read every frame of a byte stream. */
async function readAll(readable: ReadableStream<Uint8Array>): Promise<WebSocketFrame[]> {
    const frames: WebSocketFrame[] = [];
    for await (const frame of WebSocketFrame.read(readable)) {
        frames.push(frame);
    }

    return frames;
}

test("roundtrip text, binary and close frames of every length form, whole or a byte at a time", async () => {
    // frame a binary message at each length edge, a text message and a close
    const binaries = LENGTHS.map((length) =>
        WebSocketFrame.message(new Uint8Array(length).map((_, index) => index % 251)),
    );
    const frames = [
        ...binaries,
        WebSocketFrame.message("hello"),
        WebSocketFrame.close(1000, "done"),
    ];
    const bytes = joined(frames);

    // read them back whole and a byte at a time
    const whole = await readAll(chunked(bytes, bytes.length));
    const bytewise = await readAll(chunked(bytes, 1));

    // expect the same frames each way, and the close's code and reason
    expect({ whole, bytewise, closing: whole.at(-1)?.closing() }).toEqual({
        whole: frames,
        bytewise: frames,
        closing: { code: 1000, reason: "done" },
    });
});

test("send a close no peer sends as going away", () => {
    // frame the abnormal closure a socket reports after a lost connection
    const frame = WebSocketFrame.close(1006, "lost");

    expect([frame.opcode, frame.closing()]).toEqual([
        WebSocketOpcode.close,
        { code: 1001, reason: "lost" },
    ]);
});

test("refuse a masked frame and a stream that ends inside a frame", async () => {
    // set the mask bit of a text frame, and cut another short
    const masked = WebSocketFrame.message("hi").encode();
    masked[1] = (masked[1] ?? 0) | 0x80;
    const cut = WebSocketFrame.message("hello").encode().subarray(0, 4);

    expect([
        await refusal(readAll(chunked(masked, masked.length))),
        await refusal(readAll(chunked(cut, cut.length))),
    ]).toEqual([
        [
            "BAD_REQUEST",
            "PROTOCOL_VIOLATION_INVALID",
            "websocket frame is invalid: a tunnel carries whole, unmasked text, binary or close frames",
        ],
        [
            "BAD_REQUEST",
            "PROTOCOL_VIOLATION_INVALID",
            "websocket frame is invalid: the stream ended inside it",
        ],
    ]);
});
