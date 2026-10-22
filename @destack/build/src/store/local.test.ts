import { exists, mkdir, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { LocalBucket } from "@destack/bucket/bun";
import { Clock } from "@destack/environment";
import { Package } from "@destack/package";
import { BuildReader, BuildWriter } from "@destack/package/manifest";
import { found } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { Identities, identifyToolchain } from "../cache/index.ts";
import { LocalPackageStore } from "./local.ts";

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

test("keep one store per toolchain, deleting other toolchains' idle stores and reusing its own", async () => {
    // write an idle store and an open one of other toolchains, beside a file a file manager left
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const idle = await LocalBucket.open(join(directory, "idle"), "space-test", Clock.system);
    await idle[Symbol.asyncDispose]();
    await using open = await LocalBucket.open(join(directory, "open"), "space-test", Clock.system);
    await writeFile(join(directory, ".DS_Store"), "");

    // write an object into the toolchain's store, then reopen it
    {
        await using store = await LocalPackageStore.open(directory);
        await store.bucket.put("marker", "kept");
    }
    await using reopened = await LocalPackageStore.open(directory);

    // keep the open store and the toolchain's own, which still has the object
    const toolchain = await identifyToolchain(Identities.of({}));
    expect({
        stores: (await readdir(directory)).toSorted(),
        marker: await (await reopened.bucket.get("marker"))?.text(),
    }).toEqual({
        stores: [toolchain, ".DS_Store", "open", "stores.lock"].toSorted(),
        marker: "kept",
    });
});

test("share the toolchain's store between opens, each reading what the other writes", async () => {
    // open the store twice at once, as two test processes do
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await using first = await LocalPackageStore.open(directory);
    await using second = await LocalPackageStore.open(directory);

    // read through each open what the other wrote
    await first.bucket.put("first", "from first");
    await second.bucket.put("second", "from second");
    expect({
        first: await (await second.bucket.get("first"))?.text(),
        second: await (await first.bucket.get("second"))?.text(),
    }).toEqual({ first: "from first", second: "from second" });
});

test("keep a build's files while another process opens the store twice before its manifest", async () => {
    // put the files of a build in flight, as a compiler stores them before the manifest
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await using building = await LocalPackageStore.open(directory);
    const build = await written("export const answer = 42;\n");
    const files = await build.distributed();
    for (const file of files) {
        await building.putFile(file, () => build.stream(file.path));
    }

    // open the store twice elsewhere, which would mark and then delete unlisted files of an idle store
    for (const attempt of ["marking", "deleting"]) {
        await using opened = await LocalPackageStore.open(directory);
        expect({ attempt, trees: opened.trees }).toEqual({ attempt, trees: building.trees });
    }

    // keep every file the build put before its manifest
    const kept = await Promise.all(
        files.map(async (file) => (await building.bucket.head(`files/${file.digest}`)) !== null),
    );
    expect(kept).toEqual(files.map(() => true));
});

test("collect an unrooted build and its tree on the second idle open, keeping a rooted one", async () => {
    // store a rooted build and an unrooted one with its tree
    const directory = await mkdtemp(join(tmpdir(), "destack-test-stores-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    let rooted: string;
    let dropped: string;
    {
        await using store = await LocalPackageStore.open(directory);
        rooted = await store.put(await written("export const answer = 42;\n"));
        dropped = await store.put(await written("export const answer = 43;\n"));
        await store.root("answer", new Set([rooted]));
        await mkdir(join(store.trees, dropped, "node_modules"), { recursive: true });
        await mkdir(join(store.trees, rooted, "node_modules"), { recursive: true });
    }

    // mark the unrooted build on one idle open and delete it on the next
    {
        await using marking = await LocalPackageStore.open(directory);
        void marking;
    }
    await using store = await LocalPackageStore.open(directory);
    expect({
        rooted: await store.contains(rooted),
        dropped: await store.contains(dropped),
        rootedTree: await exists(join(store.trees, rooted)),
        droppedTree: await exists(join(store.trees, dropped)),
    }).toEqual({ rooted: true, dropped: false, rootedTree: true, droppedTree: false });
});
