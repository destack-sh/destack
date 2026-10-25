import { testSimulation } from "../test/world.ts";
import { expect, test } from "@destack/test";
import { createClient } from "../client/index.ts";
import { health } from "../health/index.ts";
import { Health } from "./health.ts";
import { implementHealth, RequestHandler } from "./index.ts";
import { CallError } from "../error/index.ts";
import { fixturePackage } from "../test/fixture.ts";

test("stream current health and subsequent readiness changes", async () => {
    const world = testSimulation().environment("test");
    // serve the health procedures
    const readiness = new Health("notes");
    const handler = new RequestHandler<{ caller: string; request: Request }>(
        {
            health: implementHealth(readiness),
        },
        {
            package: fixturePackage,
            environment: world,
            health: readiness,
            authorize: async ({ request: state }) => {
                if (state.caller !== "alice") {
                    throw CallError.http("UNAUTHORIZED", { message: "unknown caller" });
                }
            },
        },
    );

    // connect a client
    const client = createClient(
        { router: { health } },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const result = await handler.handle(request, {
                    context: { caller: "alice", request },
                });

                return result.matched ? result.response : new Response(null, { status: 404 });
            },
        },
        world,
    );

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
