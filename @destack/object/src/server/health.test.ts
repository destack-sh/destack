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
    readiness.set("up", "the notes are open");
    expect(await stream.next()).toEqual({
        done: false,
        value: { name: "notes", status: "up", reason: "the notes are open" },
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
    serving.set("up", "the notes are open");
    failing.set("down", "the files' database is missing");

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
            "SERVER_DOWN",
            "files is down: the files' database is missing",
        ],
        probe: [503, "down: the files' database is missing\n"],
    });
});

test("answer a starting server's readiness probe with when to ask again", () => {
    // probe a server still starting, and one that is down
    const starting = new Health("notes");
    const down = new Health("files");
    starting.set("starting", "the notes open");
    down.set("down", "the files' database is missing");
    // ask the starting one to retry after a second, and the down one never
    expect([answer(starting), answer(down)]).toEqual([
        [503, "1"],
        [503, null],
    ]);
});

/** Probe a server's readiness, as its status and its Retry-After. */
function answer(readiness: Health) {
    const answered = readiness.probe(new Request("https://test.local/readyz"));

    return [answered?.status, answered?.headers.get("retry-after")];
}
