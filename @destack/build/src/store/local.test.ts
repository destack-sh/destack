import { TEST_START } from "@destack/test/world";
import type { Crypto } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { LocalBucket } from "@destack/bucket/bun";
import { Simulation } from "@destack/environment/simulation";
import { Package } from "@destack/package";
import { BuildReader, BuildWriter } from "@destack/package/manifest";
import { type Digest, found } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { Identities } from "../cache/index.ts";
import { testSimulation } from "../test/world.ts";
import { LocalPackageStore } from "./local.ts";

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60_000;

/** A week and a moment, past which an idle open sweeps an unrooted build. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000 + 1;

/** Write a build of one module in memory, as a builder writes one. */
async function written(text: string, crypto: Crypto): Promise<BuildReader> {
    // write the module, then the format listing it
    const bytes = new Map<string, Uint8Array<ArrayBuffer>>();
    const writer = new BuildWriter(
        {
            write: async (path, contents) => {
                bytes.set(path, contents);
            },
        },
        crypto,
    );
    await writer.write("src/answer.ts", new TextEncoder().encode(text));
    const manifest = await writer.finish({
        package: Package.parse({
            id: "package-00000000-0000-7000-8000-00000000a001",
            name: "@example/answer",
            version: "2026.9.0",
        }),
    });

    return new BuildReader(
        manifest,
        async (path) => new Blob([found(bytes, path)]).stream(),
        crypto,
    );
}

test("open one store per toolchain, deleting other toolchains' idle stores and reusing the current one", async () => {
    // write an idle store and an open one of other toolchains, beside a file a file manager left
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    const idle = await LocalBucket.open(join(directory, "idle"), "space-test", environment);
    await idle[Symbol.asyncDispose]();
    await using _open = await LocalBucket.open(join(directory, "open"), "space-test", environment);
    await files.writeTextFile(join(directory, ".DS_Store"), "");

    // write an object into the toolchain's store, then reopen it
    {
        await using store = await LocalPackageStore.open(directory, environment);
        await store.bucket.put("marker", "stored");
    }
    await using reopened = await LocalPackageStore.open(directory, environment);

    // retain the open store and the toolchain's store, which still has the object
    const toolchain = await Identities.of({}, environment).toolchain();
    expect({
        stores: (await files.readDirectory(directory)).map((entry) => entry.name).toSorted(),
        marker: await (await reopened.bucket.get("marker"))?.text(),
    }).toEqual({
        stores: [toolchain, ".DS_Store", "open", "stores.lock"].toSorted(),
        marker: "stored",
    });
});

test("share the toolchain's store between opens, each reading what the other writes", async () => {
    // open the store twice at once, as two test processes do
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using first = await LocalPackageStore.open(directory, environment);
    await using second = await LocalPackageStore.open(directory, environment);

    // read through each open what the other wrote
    await first.bucket.put("first", "from first");
    await second.bucket.put("second", "from second");
    expect({
        first: await (await second.bucket.get("first"))?.text(),
        second: await (await first.bucket.get("second"))?.text(),
    }).toEqual({ first: "from first", second: "from second" });
});

test("retain a build's files while another process opens the store twice before its manifest", async () => {
    // put the files of a build in flight, as a compiler stores them before the manifest
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using building = await LocalPackageStore.open(directory, environment);
    const build = await written("export const answer = 42;\n", environment.crypto);
    const distributed = await build.distributed();
    for (const file of distributed) {
        await building.putFile(file, () => build.stream(file.path));
    }

    // open the store twice elsewhere, which would mark and then delete unlisted files of an idle store
    for (const attempt of ["marking", "deleting"]) {
        await using opened = await LocalPackageStore.open(directory, environment);
        expect({ attempt, trees: opened.trees }).toEqual({ attempt, trees: building.trees });
    }

    // retain every file the build put before its manifest
    const stored = await Promise.all(
        distributed.map(
            async (file) => (await building.bucket.head(`files/${file.digest}`)) !== null,
        ),
    );
    expect(stored).toEqual(distributed.map(() => true));
});

test("collect an unrooted build and its tree a week after storing it, on the next day's idle open", async () => {
    // store a rooted build and an unrooted one with its tree
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    const { clock } = simulation;
    let rooted: Digest;
    let dropped: Digest;
    {
        await using store = await LocalPackageStore.open(directory, environment);
        rooted = await store.put(await written("export const answer = 42;\n", environment.crypto));
        dropped = await store.put(await written("export const answer = 43;\n", environment.crypto));
        await store.root("answer", new Set([rooted]));
        await files.mkdir(join(store.trees, dropped, "node_modules"), { recursive: true });
        await files.mkdir(join(store.trees, rooted, "node_modules"), { recursive: true });
    }

    // keep the unrooted build through two idle opens within the week
    const contents = async () => {
        await using store = await LocalPackageStore.open(directory, environment);

        return {
            rooted: await store.contains(rooted),
            dropped: await store.contains(dropped),
            rootedTree: await files.exists(join(store.trees, rooted)),
            droppedTree: await files.exists(join(store.trees, dropped)),
        };
    };
    const kept = [await contents(), await contents()];

    // mark it on one idle open past the week and delete it on an open a day later
    clock.set(clock.now() + WEEK_MILLISECONDS);
    await contents();
    clock.set(clock.now() + DAY_MILLISECONDS);
    const swept = await contents();
    const all = { rooted: true, dropped: true, rootedTree: true, droppedTree: true };
    expect({ kept, swept }).toEqual({
        kept: [all, all],
        swept: { rooted: true, dropped: false, rootedTree: true, droppedTree: false },
    });
});

test("keep an unrooted build a week past its last use, and collect it once a week passed unused", async () => {
    // store an unrooted build, and use it six days later
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    const { clock } = simulation;
    let used: Digest;
    {
        await using store = await LocalPackageStore.open(directory, environment);
        used = await store.put(await written("export const answer = 44;\n", environment.crypto));
    }
    clock.set(clock.now() + DAY_MILLISECONDS * 6);
    {
        await using store = await LocalPackageStore.open(directory, environment);
        await store.contents(used);
    }

    // read whether two idle opens a day apart keep it
    const contains = async () => {
        await using store = await LocalPackageStore.open(directory, environment);

        return store.contains(used);
    };
    const opens = async () => {
        const first = await contains();
        clock.set(clock.now() + DAY_MILLISECONDS);

        return [first, await contains()];
    };

    // keep it past the week after storing it, then drop it a week after its use
    clock.set(clock.now() + DAY_MILLISECONDS * 2);
    const kept = await opens();
    clock.set(clock.now() + WEEK_MILLISECONDS);
    const dropped = await opens();
    expect({ kept, dropped }).toEqual({ kept: [true, true], dropped: [true, false] });
});

test("refuse an empty cache directory the environment sets for a checkout's store", async () => {
    // run in a world whose cache directory variable is empty
    const host = testHost();
    await using simulation = new Simulation({
        seed: 1,
        start: TEST_START,
        host: host.host,
        process: {
            arguments: [],
            variables: { DESTACK_CACHE_DIRECTORY: "" },
            workingDirectory: "/",
            id: 1,
            userId: host.process.userId,
            executable: host.process.executable,
            runtime: host.process.runtime,
        },
    });
    const environment = simulation.environment("test");

    // refuse to open the store
    const directory = fileURLToPath(new URL(".", import.meta.url));
    expect(await refusal(LocalPackageStore.checkout(directory, "test", environment))).toEqual([
        "BAD_REQUEST",
        "CACHE_DIRECTORY_INVALID",
        "DESTACK_CACHE_DIRECTORY is empty",
    ]);
});
