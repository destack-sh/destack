import { testSimulation } from "@destack/test/world";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { LocalBucket } from "./index.ts";

test("time a catalogue lock wait only when another operation holds the lock", async () => {
    // write alone, then read and list at once, so the listing waits behind the read
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-lock-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    await bucket.put("notes/first.txt", "Hello");
    const [file] = await Promise.all([
        bucket.get("notes/first.txt"),
        bucket.list({ prefix: "notes/" }),
    ]);
    await present(file, "the file").text();

    // expect one timed wait: the listing's, the write and the read taking a free lock
    const counts = await simulation.telemetry.counts();
    expect(counts["destack.bucket.lock.wait_time.count"]).toBe(1);
});
