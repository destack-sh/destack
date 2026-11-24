import { MemoryBucket } from "@destack/bucket/test";
import { SimulatedClock } from "@destack/environment/simulation";
import { expect, onTestFinished, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { TestStore } from "./store.ts";

/** The time the store's clock starts at, in epoch milliseconds. */
const START = 1_795_000_000_000;

/** Open two computers' stores over one bucket on a paused clock, each with its own directory. */
async function computers() {
    // share one bucket and one clock
    const host = testHost();
    const clock = new SimulatedClock(START);
    const bucket = new MemoryBucket(clock);
    const environment = { ...host, clock };

    // give each computer its own package directory
    const directories = await Promise.all(
        ["first", "second"].map(async (name) => {
            const directory = await host.files.makeTemporaryDirectory(`store-${name}-`);
            onTestFinished(() => host.files.remove(directory, { recursive: true }));

            return directory;
        }),
    );

    return { clock, bucket, store: new TestStore(bucket, environment), directories };
}

test("read the files a run passed over a closure on another computer, and none over another closure", async () => {
    // pass two files on the first computer
    const { store, directories } = await computers();
    const [first = "", second = ""] = directories;
    await store.pass(
        first,
        "@example/notes",
        "closure-a",
        new Map([
            ["a.test.ts", 10],
            ["b.test.ts", 20],
        ]),
    );

    // read them on both computers over the same closure, and nothing over another
    expect([...(await store.passed(first, "@example/notes", "closure-a"))].toSorted()).toEqual([
        "a.test.ts",
        "b.test.ts",
    ]);
    expect([...(await store.passed(second, "@example/notes", "closure-a"))].toSorted()).toEqual([
        "a.test.ts",
        "b.test.ts",
    ]);
    expect([...(await store.passed(second, "@example/notes", "closure-b"))]).toEqual([]);
});

test("read each file's duration from the newest record that has it, over any closure", async () => {
    // pass two files, then one of them again later over another closure
    const { clock, store, directories } = await computers();
    const [first = ""] = directories;
    await store.pass(
        first,
        "@example/notes",
        "closure-a",
        new Map([
            ["a.test.ts", 10.4],
            ["b.test.ts", 20],
        ]),
    );
    clock.advance(1_000);
    await store.pass(first, "@example/notes", "closure-b", new Map([["a.test.ts", 30]]));

    // take the newer duration of the file run twice, rounded, and the older one of the other
    expect(Object.fromEntries(await store.durations("@example/notes"))).toEqual({
        "a.test.ts": 30,
        "b.test.ts": 20,
    });
});

test("read the newest check duration, and none for a package never checked", async () => {
    // check a package twice
    const { clock, store } = await computers();
    await store.check("@example/notes", 4_000);
    clock.advance(1_000);
    await store.check("@example/notes", 5_000);

    // read the second check's duration
    expect(await store.checked("@example/notes")).toBe(5_000);
    expect(await store.checked("@example/tasks")).toBeUndefined();
});

test("keep records on this computer alone without the store's variables, and refuse a partial set", async () => {
    // open the store without any variable
    const host = testHost();
    const alone = await TestStore.open({ ...host, process: { ...host.process, variables: {} } });
    expect(alone.bucket).toBeUndefined();

    // refuse a store named without its token
    const partial = TestStore.open({
        ...host,
        process: { ...host.process, variables: { DESTACK_CI_STORE: "https://example.com/ci" } },
    });
    await expect(partial).rejects.toThrow(
        "the test store needs DESTACK_CI_STORE, DESTACK_CI_STORE_KEY_ID and DESTACK_CI_STORE_TOKEN together",
    );
});
