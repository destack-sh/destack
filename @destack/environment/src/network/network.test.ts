import { expect, test } from "@destack/test";
import { BunEnvironment } from "../bun.ts";
import { Network } from "./network.ts";

test("stream a body through the browser network, keeping its status and headers, until the request aborts", async () => {
    // serve a body that sends a chunk and never ends
    const { server } = BunEnvironment.system;
    const listener = server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () =>
            new Response(
                new ReadableStream<Uint8Array>({
                    start: (controller) => controller.enqueue(new TextEncoder().encode("first")),
                }),
                { status: 202, headers: { "x-stream": "open" } },
            ),
    });

    // read the first chunk, then abort the request and read again
    try {
        const aborting = new AbortController();
        const response = await Network.browser.fetch(
            new Request(listener.url, { signal: aborting.signal }),
        );
        if (response.body === null) {
            throw new TypeError("the response has no body");
        }
        const reader = response.body.getReader();
        const first = await reader.read();
        aborting.abort();
        const next = await reader.read().then(
            () => "read",
            (error: unknown) => (error instanceof Error ? error.name : String(error)),
        );

        // the response keeps its status and headers, and the read after the abort fails with it
        expect({
            status: response.status,
            header: response.headers.get("x-stream"),
            first: new TextDecoder().decode(first.value),
            next,
        }).toEqual({ status: 202, header: "open", first: "first", next: "AbortError" });
    } finally {
        await listener.stop(true);
    }
});
