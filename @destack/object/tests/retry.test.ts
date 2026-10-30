import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { Simulation } from "@destack/environment/simulation";
import { nth } from "@destack/schema";
import { ObjectClient } from "../src/client/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/note.ts";
import { unmoved } from "./fixture/space.ts";

test("push again after transient failures, waiting twice as long after each", async () => {
    const simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 1) });
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
        endpoint: { ...alice, fetch },
        reconnect: unmoved,
        retry: { initialInterval: 20 },
    });
    client.query.note.findMany().subscribe();
    const reported: unknown[] = [];
    const stopping = new AbortController();
    const running = client.run(stopping.signal, (error) => reported.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note after two reported failures with doubling retry waits, moving time for them
    const created = client.mutate(note).create({ title: "Ideas" });
    await created.confirmed;
    const waits = attempts.slice(1).map((at, position) => at - nth(attempts, position));
    expect([
        reported.map((error) => (error instanceof Error ? error.message : error)),
        attempts.length,
        nth(waits, 0) >= 20 && nth(waits, 1) >= 40,
    ]).toEqual([["fetch failed", "fetch failed"], 3, true]);
});
