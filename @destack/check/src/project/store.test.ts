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

test("read the files a run passed over the same inputs on another computer, and none over other inputs", async () => {
    // pass two files on the first computer
    const { store, directories } = await computers();
    const [first = "", second = ""] = directories;
    await store.pass(
        first,
        "@example/notes",
        "inputs-a",
        new Map([
            ["a.test.ts", 10],
            ["b.test.ts", 20],
        ]),
    );

    // read them on both computers over the same inputs, and nothing over another
    expect([...(await store.passed(first, "@example/notes", "inputs-a"))].toSorted()).toEqual([
        "a.test.ts",
        "b.test.ts",
    ]);
    expect([...(await store.passed(second, "@example/notes", "inputs-a"))].toSorted()).toEqual([
        "a.test.ts",
        "b.test.ts",
    ]);
    expect([...(await store.passed(second, "@example/notes", "inputs-b"))]).toEqual([]);
});

test("read each file's duration from the newest record that has it, over any inputs", async () => {
    // pass two files, then one of them again later over other inputs
    const { clock, store, directories } = await computers();
    const [first = ""] = directories;
    await store.pass(
        first,
        "@example/notes",
        "inputs-a",
        new Map([
            ["a.test.ts", 10.4],
            ["b.test.ts", 20],
        ]),
    );
    clock.advance(1_000);
    await store.pass(first, "@example/notes", "inputs-b", new Map([["a.test.ts", 30]]));

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

test("report the writes a bucket refuses, recording the passes on this computer", async () => {
    // pass a file and record a check over a bucket refusing every write
    const host = testHost();
    const clock = new SimulatedClock(START);
    const refusing = new (class extends MemoryBucket {
        /** Refuse every write, as an unreachable bucket does. */
        override async put(): Promise<never> {
            throw new Error("bucket is unreachable");
        }
    })(clock);
    const written: string[] = [];
    const decoder = new TextDecoder();
    const stderr = {
        ...host.process.stderr,
        writeSync: (chunk: string | Uint8Array) => {
            written.push(typeof chunk === "string" ? chunk : decoder.decode(chunk));
        },
    };
    const environment = { ...host, clock, process: { ...host.process, stderr } };
    const directory = await host.files.makeTemporaryDirectory("store-refusing-");
    onTestFinished(() => host.files.remove(directory, { recursive: true }));
    const store = new TestStore(refusing, environment);
    await store.pass(directory, "@example/notes", "inputs-a", new Map([["a.test.ts", 10]]));
    await store.check("@example/notes", 1000);

    // expect both refusals reported by key with the random identifier masked, and the pass on this computer
    const local = new TestStore(undefined, environment);
    const reported = written.map((line) =>
        line.replace(/[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}/u, "<id>"),
    );
    expect([reported, await local.passed(directory, "@example/notes", "inputs-a")]).toEqual([
        [
            "cannot record 'tests/@example/notes/inputs-a/<id>.json': bucket is unreachable\n",
            "cannot record 'checks/@example/notes/<id>.json': bucket is unreachable\n",
        ],
        new Set(["a.test.ts"]),
    ]);
});
