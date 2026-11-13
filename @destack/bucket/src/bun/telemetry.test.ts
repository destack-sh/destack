import { testSimulation } from "@destack/test/world";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { LocalBucket } from "./index.ts";

/** The instrumentation scope of the bucket's signals. */
const SCOPE = "@destack/bucket";

test("trace a local bucket's open, write, read, listing and deletion, and time a real lock wait", async () => {
    // open a bucket, write a file, read and list it at once, so one waits for the other, then delete it
    await using simulation = testSimulation();
    const world = simulation.environment("host");
    const directory = await world.files.makeTemporaryDirectory("bucket-telemetry-");
    await using bucket = await LocalBucket.open(directory, "space-test", world);
    await bucket.put("notes/first.txt", "Hello");
    const [file] = await Promise.all([
        bucket.get("notes/first.txt"),
        bucket.list({ prefix: "notes/" }),
    ]);
    await present(file, "the file").text();
    await bucket.delete("notes/first.txt");

    // read the bucket's spans in the order they ended, and its own counts
    const spans = simulation.telemetry.spans
        .filter((span) => span.instrumentationScope.name === SCOPE)
        .map((span) => span.name);
    const counts = Object.fromEntries(
        Object.entries(await simulation.telemetry.counts()).filter(
            ([key]) => key.startsWith("destack.bucket.") && key.endsWith(".count"),
        ),
    );

    // expect each verb once, the listing ending before the read opens its body, and the listing's one lock wait
    expect({ spans, counts }).toEqual({
        spans: [
            "destack.bucket.open",
            "destack.bucket.put",
            "destack.bucket.list",
            "destack.bucket.get",
            "destack.bucket.delete",
        ],
        counts: {
            "destack.bucket.open.duration.count": 1,
            "destack.bucket.put.duration.count": 1,
            "destack.bucket.get.duration.count": 1,
            "destack.bucket.list.duration.count": 1,
            "destack.bucket.delete.duration.count": 1,
            "destack.bucket.lock.wait_time.count": 1,
        },
    });
});
