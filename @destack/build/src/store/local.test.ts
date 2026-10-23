import { exists, mkdir, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { LocalBucket } from "@destack/bucket/bun";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";
import { Package } from "@destack/package";
import { BuildReader, BuildWriter } from "@destack/package/manifest";
import { found } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Identities, identifyToolchain } from "../cache/index.ts";
import { LocalPackageStore } from "./local.ts";

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60_000;

/** A week and a moment, past which an idle open sweeps an unrooted build. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000 + 1;

/** Write a build of one module in memory, as a builder writes one. */
async function written(text: string): Promise<BuildReader> {
    // write the module, then the format listing it
    const bytes = new Map<string, Uint8Array<ArrayBuffer>>();
    const writer = new BuildWriter({
        write: async (path, contents) => {
            bytes.set(path, contents);
        },
    });
    await writer.write("src/answer.ts", new TextEncoder().encode(text));
    const manifest = await writer.finish({
        package: Package.parse({
            id: "package-00000000-0000-7000-8000-00000000a001",
            name: "@example/answer",
            version: "2026.9.0",
        }),
    });

    return new BuildReader(manifest, async (path) => new Blob([found(bytes, path)]).stream());
}

test("open one store per toolchain, deleting other toolchains' idle stores and reusing the current one", async () => {
    // write an idle store and an open one of other toolchains, beside a file a file manager left
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const idle = await LocalBucket.open(join(directory, "idle"), "space-test", Clock.system);
    await idle[Symbol.asyncDispose]();
    await using _open = await LocalBucket.open(join(directory, "open"), "space-test", Clock.system);
    await writeFile(join(directory, ".DS_Store"), "");

    // write an object into the toolchain's store, then reopen it
    {
        await using store = await LocalPackageStore.open(directory, Clock.system);
        await store.bucket.put("marker", "stored");
    }
    await using reopened = await LocalPackageStore.open(directory, Clock.system);

    // retain the open store and the toolchain's store, which still has the object
    const toolchain = await identifyToolchain(Identities.of({}));
    expect({
        stores: (await readdir(directory)).toSorted(),
        marker: await (await reopened.bucket.get("marker"))?.text(),
    }).toEqual({
        stores: [toolchain, ".DS_Store", "open", "stores.lock"].toSorted(),
        marker: "stored",
    });
});

test("share the toolchain's store between opens, each reading what the other writes", async () => {
    // open the store twice at once, as two test processes do
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await using first = await LocalPackageStore.open(directory, Clock.system);
    await using second = await LocalPackageStore.open(directory, Clock.system);

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
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await using building = await LocalPackageStore.open(directory, Clock.system);
    const build = await written("export const answer = 42;\n");
    const files = await build.distributed();
    for (const file of files) {
        await building.putFile(file, () => build.stream(file.path));
    }

    // open the store twice elsewhere, which would mark and then delete unlisted files of an idle store
    for (const attempt of ["marking", "deleting"]) {
        await using opened = await LocalPackageStore.open(directory, Clock.system);
        expect({ attempt, trees: opened.trees }).toEqual({ attempt, trees: building.trees });
    }

    // retain every file the build put before its manifest
    const stored = await Promise.all(
        files.map(async (file) => (await building.bucket.head(`files/${file.digest}`)) !== null),
    );
    expect(stored).toEqual(files.map(() => true));
});

test("collect an unrooted build and its tree a week after storing it, on the second idle open", async () => {
    // store a rooted build and an unrooted one with its tree
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const clock = new VirtualClock(Clock.system.now());
    let rooted: string;
    let dropped: string;
    {
        await using store = await LocalPackageStore.open(directory, clock);
        rooted = await store.put(await written("export const answer = 42;\n"));
        dropped = await store.put(await written("export const answer = 43;\n"));
        await store.root("answer", new Set([rooted]));
        await mkdir(join(store.trees, dropped, "node_modules"), { recursive: true });
        await mkdir(join(store.trees, rooted, "node_modules"), { recursive: true });
    }

    // keep the unrooted build through two idle opens within the week
    const contents = async () => {
        await using store = await LocalPackageStore.open(directory, clock);

        return {
            rooted: await store.contains(rooted),
            dropped: await store.contains(dropped),
            rootedTree: await exists(join(store.trees, rooted)),
            droppedTree: await exists(join(store.trees, dropped)),
        };
    };
    const kept = [await contents(), await contents()];

    // mark it on one idle open past the week and delete it on the next
    clock.set(clock.now() + WEEK_MILLISECONDS);
    await contents();
    const swept = await contents();
    const all = { rooted: true, dropped: true, rootedTree: true, droppedTree: true };
    expect({ kept, swept }).toEqual({
        kept: [all, all],
        swept: { rooted: true, dropped: false, rootedTree: true, droppedTree: false },
    });
});

test("keep an unrooted build a week past its last use, and collect it once a week passed unused", async () => {
    // store an unrooted build, and use it six days later
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const clock = new VirtualClock(Clock.system.now());
    let used: string;
    {
        await using store = await LocalPackageStore.open(directory, clock);
        used = await store.put(await written("export const answer = 44;\n"));
    }
    clock.set(clock.now() + DAY_MILLISECONDS * 6);
    {
        await using store = await LocalPackageStore.open(directory, clock);
        await store.contents(used);
    }

    // read whether it is kept after two idle opens, as a sweep marks it and then deletes it
    const contains = async () => {
        await using store = await LocalPackageStore.open(directory, clock);

        return store.contains(used);
    };
    const opens = async () => [await contains(), await contains()];

    // keep it past the week after storing it, then drop it a week after its use
    clock.set(clock.now() + DAY_MILLISECONDS * 2);
    const kept = await opens();
    clock.set(clock.now() + WEEK_MILLISECONDS);
    const dropped = await opens();
    expect({ kept, dropped }).toEqual({ kept: [true, true], dropped: [true, false] });
});

test("refuse an empty cache directory the environment sets for a checkout's store", async () => {
    // set the variable empty for this test alone
    const { DESTACK_CACHE_DIRECTORY: before } = process.env;
    process.env["DESTACK_CACHE_DIRECTORY"] = "";
    onTestFinished(() => {
        if (before === undefined) {
            delete process.env["DESTACK_CACHE_DIRECTORY"];
        } else {
            process.env["DESTACK_CACHE_DIRECTORY"] = before;
        }
    });

    // refuse to open the store
    const directory = fileURLToPath(new URL(".", import.meta.url));
    expect(await refusal(LocalPackageStore.checkout(directory, "test"))).toEqual([
        "BAD_REQUEST",
        "CACHE_DIRECTORY_INVALID",
        "DESTACK_CACHE_DIRECTORY is empty",
    ]);
});
