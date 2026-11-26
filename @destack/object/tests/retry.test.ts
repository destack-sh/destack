import { worldTelemetry } from "../src/test/world.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { Simulation } from "@destack/environment/simulation";
import { nth } from "@destack/schema";
import { type ObjectRemote } from "../src/client/index.ts";
import { ObjectClient } from "../src/device/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "../src/test/note.ts";
import { SyncError } from "@destack/sync";
import { testWorldOptions } from "@destack/test/world";

test("push again after dropped connections, waiting twice as long after each", async () => {
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: Date.UTC(2026, 9, 1),
    });
    const { connection, clock, environment } = await serveNotes({ simulation });
    const alice = connection(users.alice);

    // fail the first two pushes as a dropped connection does, recording each start
    const attempts: number[] = [];
    const remote: ObjectRemote = {
        stream: (subscription, signal) => alice.stream(subscription, signal),
        request: (subscription) => alice.request(subscription),
        push: async (home, push, signal) => {
            attempts.push(clock.now());
            if (attempts.length <= 2) {
                throw SyncError.of("CONNECTION_UNAVAILABLE", "connection is unavailable: dropped");
            }

            return await alice.push(home, push, signal);
        },
        call: (scope, call, signer) => alice.call(scope, call, signer),
        broadcast: (scope, object, event, signer) => alice.broadcast(scope, object, event, signer),
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
        remote,
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
    ]).toEqual([
        ["connection is unavailable: dropped", "connection is unavailable: dropped"],
        3,
        true,
    ]);
});
