import { expect, test } from "@destack/test";
import { createClient } from "../client/index.ts";
import { Health, health } from "../health/index.ts";
import { implementHealth, ServiceHandler } from "../server/index.ts";
import { ServiceError } from "../error/index.ts";
import { defineService } from "../declare/index.ts";

test("stream current health and subsequent readiness changes", async () => {
    // serve the health procedures
    const readiness = new Health("notes");
    const service = defineService("notes", { health });
    const handler = new ServiceHandler<{ caller: string; request: Request }>(
        {
            health: implementHealth(readiness),
        },
        {
            service,
            health: readiness,
            authorize: async ({ context }) => {
                if (context.caller !== "alice") {
                    throw new ServiceError("UNAUTHORIZED", { message: "unknown caller" });
                }
            },
        },
    );

    // connect a client
    const client = createClient(service, {
        url: "https://test.local",
        fetch: async (request) => {
            const result = await handler.handle(request, { context: { caller: "alice", request } });

            return result.matched ? result.response : new Response(null, { status: 404 });
        },
    });

    // read the initial state
    expect(await client.health.check()).toEqual({ name: "notes", status: "starting" });
    const stream = await client.health.watch();
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "starting" },
    });

    // observe a change
    readiness.set("serving");
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "serving" },
    });

    // deliver the final state
    readiness.set("stopped");
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "stopped" },
    });
    expect(await stream.next()).toEqual({ done: true, value: undefined });
});
