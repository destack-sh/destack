import { worldTelemetry } from "../src/test/world.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { Simulation } from "@destack/environment/simulation";
import { nth } from "@destack/schema";
import { ObjectClient, ProcedureRemote } from "../src/client/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "../src/test/note.ts";
import { unmoved } from "../src/test/space.ts";
import { testWorldOptions } from "@destack/test/world";

test("push again after transient failures, waiting twice as long after each", async () => {
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 1),
    });
    const { endpoint, clock, environment } = await serveNotes({ simulation });
    const alice = endpoint(users.alice);

    // fail the first two pushes as a broken network does, recording each start
    const attempts: number[] = [];
    const fetch = async (request: Request) => {
        const isPush = new URL(request.url).pathname.endsWith("/store/push");
        if (isPush) {
            attempts.push(clock.now());
        }
        if (isPush && attempts.length <= 2) {
            throw new TypeError("fetch failed");
        }

        return await alice.fetch(request);
    };

    // push with retries after 20 ms and 40 ms
    const storage = await TestDatabase.create(ObjectClient.tables({ notebook, note }), {
        environment,
        dialect: "sqlite",
    });
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { notebook, note },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        retry: { initialInterval: 20 },
        remote: ProcedureRemote.at({
            scope: spaceId,
            endpoint: { ...alice, fetch },
            reconnect: unmoved,
            environment: environment,
        }),
    });
    client.query.note.findMany().subscribe();
    const stopping = new AbortController();
    const running = client.run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note after two captured failures with doubling retry waits, moving time for them
    const created = client.mutate(note).create({ title: "Ideas" });
    await simulation.wait(created.confirmed);
    const waits = attempts.slice(1).map((at, position) => at - nth(attempts, position));
    expect([
        worldTelemetry(environment)
            .take()
            .map((captured) => captured.message),
        attempts.length,
        nth(waits, 0) >= 20 && nth(waits, 1) >= 40,
    ]).toEqual([["fetch failed", "fetch failed"], 3, true]);
});
