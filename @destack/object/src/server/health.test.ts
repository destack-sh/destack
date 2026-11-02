import { testSimulation } from "../test/world.ts";
import { expect, refusal, test } from "@destack/test";
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
    expect(await client.health.check()).toEqual({
        name: "notes",
        status: "starting",
        reason: "the server has not started yet",
    });
    const stream = await client.health.watch();
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "starting", reason: "the server has not started yet" },
    });

    // observe a change
    readiness.set("serving", "the notes are open");
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "serving", reason: "the notes are open" },
    });

    // deliver the final state
    readiness.set("stopped", "the notes closed");
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "stopped", reason: "the notes closed" },
    });
    expect(await stream.next()).toEqual({ done: true, value: undefined });
});

test("wait for a server to serve, refusing at once with its reason once it fails while starting", async () => {
    // wait on a server that serves, and on one that fails before serving
    const serving = new Health("notes");
    const failing = new Health("files");
    const served = serving.waitUp();
    const refused = refusal(failing.waitUp());
    serving.set("serving", "the notes are open");
    failing.set("not-serving", "the files' database is missing");

    // settle the first, and refuse the second with its status and reason, as its readiness probe does
    const probe = failing.probe(new Request("https://test.local/readyz"));
    expect({
        served: await served,
        refused: await refused,
        probe: [probe?.status, await probe?.text()],
    }).toEqual({
        served: undefined,
        refused: [
            "SERVICE_UNAVAILABLE",
            "SERVER_NOT_SERVING",
            "files is not-serving: the files' database is missing",
        ],
        probe: [503, "not-serving: the files' database is missing\n"],
    });
});
