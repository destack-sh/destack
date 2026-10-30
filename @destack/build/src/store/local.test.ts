import { TEST_START } from "@destack/test/world";
import type { Crypto } from "@destack/environment";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Simulation } from "@destack/environment/simulation";
import { Package } from "@destack/package";
import { BuildReader, BuildWriter } from "@destack/package/manifest";
import { Digest, found } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { testSimulation } from "../test/world.ts";
import { LocalPackageStore } from "./local.ts";
import { STAGING_DIRECTORY } from "../dependency/tree.ts";

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60_000;

/** A week and a moment, past which an idle open collects an unrooted build. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000 + 1;

/** The bytes a test stores as a blob. */
const MARKER = new TextEncoder().encode("stored");

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

test("keep one store in a directory, which a later open of any toolchain reads", async () => {
    // store a blob, then reopen the store, as a process of another toolchain does
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    const marker = await Digest.of(MARKER, environment.crypto);
    {
        await using store = await LocalPackageStore.open(directory, environment);
        await store.receive(marker, MARKER);
    }
    await using reopened = await LocalPackageStore.open(directory, environment);

    // read the blob from the store's files beside its database and locks
    const entries = await files.readDirectory(directory);
    expect({
        entries: entries
            .map((entry) => entry.name)
            .filter((name) => !name.startsWith("store.db-"))
            .toSorted(),
        isStored: await reopened.has(marker),
    }).toEqual({
        entries: ["collected", "files", "open.lock", "store.db", "store.lock", "trees"],
        isStored: true,
    });
});

test("share the store between opens, each reading what the other writes", async () => {
    // open the store twice at once, as two test processes do
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using first = await LocalPackageStore.open(directory, environment);
    await using second = await LocalPackageStore.open(directory, environment);

    // read through each open what the other stored
    const bytes = { first: new TextEncoder().encode("first"), second: MARKER };
    const digests = {
        first: await Digest.of(bytes.first, environment.crypto),
        second: await Digest.of(bytes.second, environment.crypto),
    };
    await first.receive(digests.first, bytes.first);
    await second.receive(digests.second, bytes.second);
    expect([await second.has(digests.first), await first.has(digests.second)]).toEqual([
        true,
        true,
    ]);
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
        await building.receive(file.digest, await build.load(file.path));
    }

    // open the store twice elsewhere, which would collect unleased files of an idle store
    for (const attempt of ["marking", "deleting"]) {
        await using opened = await LocalPackageStore.open(directory, environment);
        expect({ attempt, trees: opened.trees }).toEqual({ attempt, trees: building.trees });
    }

    // retain every file the build put before its manifest
    const stored = await Promise.all(distributed.map((file) => building.has(file.digest)));
    expect(stored).toEqual(distributed.map(() => true));
});

test("collect an idle store at most daily, keeping what a crashed materialization staged until then", async () => {
    // open the store idle, then leave a staged tree behind, as a crashed materialization does
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    const { clock } = simulation;
    const staged = async () => {
        await using store = await LocalPackageStore.open(directory, environment);
        const staging = join(store.trees, STAGING_DIRECTORY);
        const isStaged = await files.exists(staging);
        await files.mkdir(staging, { recursive: true });

        return isStaged;
    };
    await staged();

    // keep it through idle opens within the day, and delete it on the first idle open a day on
    const within = [await staged()];
    await simulation.jump(clock.now() + DAY_MILLISECONDS - 1);
    within.push(await staged());
    await simulation.jump(clock.now() + 1);
    expect({ within, after: await staged() }).toEqual({ within: [true, true], after: false });
});

test("collect an unrooted build and its tree on the first idle open a week after storing it", async () => {
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

    // collect it with its tree on the first idle open past the week
    await simulation.jump(clock.now() + WEEK_MILLISECONDS);
    const collected = await contents();
    const all = { rooted: true, dropped: true, rootedTree: true, droppedTree: true };
    expect({ kept, collected }).toEqual({
        kept: [all, all],
        collected: { rooted: true, dropped: false, rootedTree: true, droppedTree: false },
    });
});

test("retain an unrooted build a week past its last use, and collect it on the first idle open after", async () => {
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
    await simulation.jump(clock.now() + DAY_MILLISECONDS * 6);
    {
        await using store = await LocalPackageStore.open(directory, environment);
        await store.contents(used);
    }

    // read whether two idle opens a day apart keep it
    const contains = async () => {
        await using store = await LocalPackageStore.open(directory, environment);

        return await store.contains(used);
    };
    const opens = async () => {
        const first = await contains();
        await simulation.jump(clock.now() + DAY_MILLISECONDS);

        return [first, await contains()];
    };

    // retain it past the week after storing it, then collect it a week after its use
    await simulation.jump(clock.now() + DAY_MILLISECONDS * 2);
    const kept = await opens();
    await simulation.jump(clock.now() + WEEK_MILLISECONDS);
    const dropped = await opens();
    expect({ kept, dropped }).toEqual({ kept: [true, true], dropped: [false, false] });
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
