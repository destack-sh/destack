import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../src/client/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook } from "./fixture/notes.ts";
import { unmoved } from "./fixture/space.ts";

test("push again after transient failures, waiting twice as long after each", async () => {
    const { connect } = await serveNotes("sqlite");
    const alice = connect("alice");

    // fail the first two pushes, recording each start
    const attempts: number[] = [];
    const push = async (...parameters: Parameters<typeof alice.replica.push>) => {
        attempts.push(performance.now());
        if (attempts.length <= 2) {
            throw new TypeError("fetch failed");
        }

        return await alice.replica.push(...parameters);
    };
    const service = new Proxy(alice.replica, {
        get: (target, name) => (name === "push" ? push : Reflect.get(target, name)),
    });

    // push with retries after 20 ms and 40 ms
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables([notebook, note]));
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        database: storage.database,
        objects: [notebook, note],
        scope: spaceId,
        caller: principal.user.reference("universe", "alice"),
        service,
        reconnect: unmoved,
        retry: { initialInterval: 20 },
    });
    client.subscribe(note);
    const reported: unknown[] = [];
    const stopping = new AbortController();
    const running = client.run(stopping.signal, (error) => reported.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });

    // confirm the note after two reported failures with doubling retry waits
    const created = client.mutate(note).create({ title: "Ideas" });
    await created.confirmed;
    const waits = attempts.slice(1).map((at, position) => at - attempts[position]!);
    expect([
        reported.map((error) => (error as Error).message),
        attempts.length,
        waits[0]! >= 20 && waits[1]! >= 40,
    ]).toEqual([["fetch failed", "fetch failed"], 3, true]);
});
