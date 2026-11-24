import { fileURLToPath } from "node:url";
import { MemoryBucket } from "@destack/bucket/test";
import { SimulatedClock } from "@destack/environment/simulation";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Project } from "./project.ts";
import { Schedule } from "./schedule.ts";
import { TestStore } from "./store.ts";

/** The workspace root, whose own packages the schedule reads. */
const ROOT = fileURLToPath(new URL("../../../../", import.meta.url));

/** The time the store's clock starts at, in epoch milliseconds. */
const START = 1_795_000_000_000;

/** Open a store over an empty bucket on a paused clock. */
function emptyStore(): TestStore {
    const clock = new SimulatedClock(START);

    return new TestStore(new MemoryBucket(clock), { ...testHost(), clock });
}

test("schedule a package whose files all passed as its check alone, on one runner", async () => {
    // pass every file of this package over its closure in the bucket alone, and record its check
    const store = emptyStore();
    const project = await Project.open(
        fileURLToPath(new URL("../../", import.meta.url)),
        testHost(),
    );
    const { TestPlan } = await import("@destack/test/plan");
    const planned = await TestPlan.of(project.directory, testHost().files);
    const durations = new Map(project.files(planned, {}).map((file) => [file, 100]));
    const scratch = await testHost().files.makeTemporaryDirectory("schedule-");
    onTestFinished(() => testHost().files.remove(scratch, { recursive: true }));
    await store.pass(scratch, "@destack/check", await project.closure({}), durations);
    await store.check("@destack/check", 9_000);

    // schedule the package: one runner checks it and finds no file to run
    const schedule = await Schedule.of(ROOT, ["@destack/check"], store, testHost());
    expect(schedule.runners).toEqual([
        {
            runner: 1,
            milliseconds: 9_000,
            work: [{ package: "@destack/check", files: [], isChecked: true }],
        },
    ]);
});

test("refuse to schedule from a store without any duration", async () => {
    const schedule = Schedule.of(ROOT, ["@destack/check"], emptyStore(), testHost());

    await expect(schedule).rejects.toThrow(
        "the test store has no durations to schedule by: record a full run first",
    );
});
