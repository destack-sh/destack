import { expect, test } from "@destack/test";
import { BunServer } from "./bun.ts";

/** The pause between two events, past Bun's 10 s idle timeout. */
const PAUSE_MILLISECONDS = 11_000;

/** The messages a socket sends, 48 MiB in all: three times Bun's 16 MiB buffer limit. */
const MESSAGE_COUNT = 48;

/** The bytes of each message: 1 MiB. */
const MESSAGE_BYTES = 1024 * 1024;

test("keep an event stream open past Bun's idle timeout", async () => {
    // stream two events a pause apart, past the idle timeout
    const encoder = new TextEncoder();
    const listener = BunServer.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () =>
            new Response(
                new ReadableStream({
                    async start(controller) {
                        controller.enqueue(encoder.encode("data: first\n\n"));
                        await Bun.sleep(PAUSE_MILLISECONDS);
                        controller.enqueue(encoder.encode("data: second\n\n"));
                        controller.close();
                    },
                }),
                { headers: { "content-type": "text/event-stream" } },
            ),
    });

    // read both events in full
    try {
        const response = await fetch(listener.url);
        expect(await response.text()).toEqual("data: first\n\ndata: second\n\n");
    } finally {
        await listener.stop(true);
    }
});

test("send past Bun's buffer limit without a drop, each send waiting for the socket to drain", async () => {
    // send every message as the socket opens, each after the previous one's buffer took it
    const sent = Promise.withResolvers<void>();
    const listener = BunServer.http<undefined>({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => ({ attachment: undefined }),
        websocket: {
            open: (socket) => {
                const sending = async () => {
                    for (let index = 0; index < MESSAGE_COUNT; index++) {
                        await socket.send(new Uint8Array(MESSAGE_BYTES).fill(index));
                    }
                };
                sending().then(sent.resolve, sent.reject);
            },
            message: () => {},
        },
    });

    // receive every message, reading each one's first byte
    try {
        const firsts: number[] = [];
        const received = Promise.withResolvers<void>();
        const socket = new WebSocket(listener.url.href.replace(/^http/u, "ws"));
        socket.binaryType = "arraybuffer";
        socket.addEventListener("message", (event: MessageEvent<ArrayBuffer>) => {
            firsts.push(new Uint8Array(event.data)[0] ?? -1);
            if (firsts.length === MESSAGE_COUNT) {
                received.resolve();
            }
        });
        await Promise.all([sent.promise, received.promise]);
        socket.close();

        expect(firsts).toEqual(Array.from({ length: MESSAGE_COUNT }, (_, index) => index));
    } finally {
        await listener.stop(true);
    }
});
