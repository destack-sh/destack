import { expect, test } from "@destack/test";
import { BunServer } from "./bun.ts";

/** The pause between two events, past Bun's 10 s idle timeout. */
const PAUSE_MILLISECONDS = 11_000;

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
