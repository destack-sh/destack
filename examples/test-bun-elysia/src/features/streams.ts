import { Elysia } from "elysia";

export const streamsFeature = new Elysia({ name: "streams" }).get(
    "/streams/time",
    () => {
        const encoder = new TextEncoder();
        let timer: ReturnType<typeof setInterval> | undefined;

        const stream = new ReadableStream<Uint8Array>({
            start(controller) {
                controller.enqueue(encoder.encode(`data: stream-start\n\n`));
                let ticks = 0;

                timer = setInterval(() => {
                    ticks += 1;
                    controller.enqueue(
                        encoder.encode(`data: ${new Date().toISOString()}\n\n`)
                    );

                    if (ticks >= 5) {
                        clearInterval(timer);
                        controller.enqueue(encoder.encode("data: done\n\n"));
                        controller.close();
                    }
                }, 1000);
            },
            cancel() {
                if (timer) clearInterval(timer);
            }
        });

        return new Response(stream, {
            headers: {
                "cache-control": "no-cache",
                "content-type": "text/event-stream"
            }
        });
    }
);
