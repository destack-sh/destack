import { TEST_START } from "@destack/test/world";
import type { Crypto } from "@destack/environment";
import { join } from "node:path";
import { Simulation } from "@destack/environment/simulation";
import { type DependencyRelease, Package } from "@destack/package";
import { BuildReader, BuildWriter, PackageManifest } from "@destack/package/manifest";
import { Digest, found, present } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { testSimulation } from "../test/world.ts";
import { LocalBuildStore } from "./local.ts";
import { BuildStore } from "./store.ts";
import { BuildLayer } from "./layer.ts";
import { DependencyTree, STAGING_DIRECTORY } from "../dependency/tree.ts";

/** A day, in milliseconds. */
const DAY_MILLISECONDS = 24 * 60 * 60_000;

/** A week and a moment, past which an idle open collects an unrooted build. */
const WEEK_MILLISECONDS = 7 * 24 * 60 * 60_000 + 1;

/** The package most builds here build. */
const ANSWER = { id: "package-00000000-0000-7000-8000-00000000a001", name: "@example/answer" };

/** The bytes a test stores as a blob. */
const MARKER = new TextEncoder().encode("stored");

/** Write a build of one module in memory, as a builder writes one, of a package with its pins. */
async function written(
    text: string,
    crypto: Crypto,
    built: { readonly id: string; readonly name: string } = ANSWER,
    dependencies: Readonly<Record<string, DependencyRelease>> = {},
    extra: Readonly<Record<string, string>> = {},
): Promise<BuildReader> {
    // write the module importing every dependency, the extra files and its package file, then the format listing them
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
    for (const [path, contents] of Object.entries(extra)) {
        await writer.write(path, new TextEncoder().encode(contents));
    }
    await writer.write(
        "package.json",
        new TextEncoder().encode(JSON.stringify({ name: built.name, exports: {} })),
    );
    const manifest = await writer.finish({ destack: "2026.11.0", role: "app",
        package: Package.parse({ ...built, version: "2026.9.0" }),
        outputs: {
            bun: {
                runtime: "bun",
                emit: true,
                workloads: {},
                views: {},
                directory: "src",
                exports: { ".": { path: "src/answer.ts", imports: Object.keys(dependencies) } },
                dependencies,
                peers: {},
            },
        },
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
        await using store = await LocalBuildStore.open(directory, environment);
        await store.receive([{ digest: marker, bytes: MARKER }]);
    }
    await using reopened = await LocalBuildStore.open(directory, environment);

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

test("place a stored file by its blob and a layered one from where its layer keeps it", async () => {
    // store a blob, and layer a build another store wrote to a distribution over the same store
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, crypto } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using store = await LocalBuildStore.open(join(directory, "store"), environment);
    const marker = await Digest.of(MARKER, crypto);
    await store.receive([{ digest: marker, bytes: MARKER }]);
    const build = await written("export const answer = 42;", crypto);
    {
        await using distributor = await LocalBuildStore.temporary(environment);
        await distributor.writeLayer(
            [await distributor.put(build)],
            [],
            join(directory, "distribution"),
        );
    }
    const layer = await BuildLayer.open(join(directory, "distribution"), environment);
    await using layered = await LocalBuildStore.of({
        blobs: store.blobs,
        directory: join(directory, "layered"),
        environment,
        layer,
    });

    // write each to a fresh folder and read the bytes back
    const module = await build.file("src/answer.ts");
    const placed = {
        stored: join(directory, "out", "stored"),
        layered: join(directory, "out", "src", "answer.ts"),
    };
    await layered.place([
        {
            file: {
                path: "stored",
                digest: marker,
                size: MARKER.byteLength,
                mediaType: "text/plain",
            },
            destination: placed.stored,
        },
        { file: module, destination: placed.layered },
    ]);
    expect([
        await files.readTextFile(placed.stored),
        await files.readTextFile(placed.layered),
    ]).toEqual(["stored", "export const answer = 42;"]);
});

test("lay out a distribution's release trees once, cloning a root and the trees it links, which outlive the layer", async () => {
    // write a layer of a stored release to a directory, as a distribution does
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, crypto } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using store = await LocalBuildStore.open(join(directory, "store"), environment);
    const build = await written("export const answer = 42;", crypto);
    const release = await store.put(build);
    const distribution = join(directory, "distribution");
    await store.writeLayer([release], [], distribution);

    // ship an npm tree beside it, which its root reaches through a relative link alone
    const npm = join(distribution, "trees", "npm-left-pad", "node_modules", "left-pad");
    await files.mkdir(npm, { recursive: true });
    await files.writeTextFile(join(npm, "index.js"), "export const pad = 1;");
    await files.symlink(
        "../../../npm-left-pad/node_modules/left-pad",
        join(distribution, "trees", release, "node_modules", "@example", "left-pad"),
    );

    // materialize the release over the opened layer
    const layer = await BuildLayer.open(distribution, environment);
    await using layered = await LocalBuildStore.of({
        blobs: store.blobs,
        directory: join(directory, "layered"),
        environment,
        layer,
    });
    const materialize = () =>
        DependencyTree.materialize(
            layered,
            [
                {
                    kind: "destack",
                    package: PackageManifest.package(build.manifest),
                    manifest: release,
                },
            ],
            join(directory, "tree"),
        );
    const tree = await materialize();
    const root = join(directory, "tree", release);
    const module = join(present(tree.directories["@example/answer"], "answer"), "src/answer.ts");

    // clone the root and the npm tree, and read both modules once the layer is removed, materializing again from the clones
    const shipped = layer.tree(release);
    const { kind } = await files.lstat(root);
    await files.remove(distribution, { recursive: true });
    await materialize();
    expect({
        shipped,
        kind,
        module: await files.readTextFile(module),
        linked: await files.readTextFile(join(root, "node_modules/@example/left-pad/index.js")),
    }).toEqual({
        shipped: join(distribution, "trees", release),
        kind: "directory",
        module: "export const answer = 42;",
        linked: "export const pad = 1;",
    });
});

test("ship a build in a layer without its output's maps, reading a map through the store's blobs", async () => {
    // store a release whose output has a map, and write a layer of it
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, crypto } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using store = await LocalBuildStore.open(join(directory, "store"), environment);
    const map = '{"version":3,"sources":["answer.ts"],"mappings":""}';
    const build = await written(
        "export const answer = 42;",
        crypto,
        ANSWER,
        {},
        {
            "src/answer.ts.map": map,
        },
    );
    const release = await store.put(build);
    const distribution = join(directory, "distribution");
    await store.writeLayer([release], [], distribution);
    await using layered = await LocalBuildStore.of({
        blobs: store.blobs,
        directory: join(directory, "layered"),
        environment,
        layer: await BuildLayer.open(distribution, environment),
    });

    // ship the module without the map, and read the map from the blobs
    const read = await layered.contents(release);
    expect({
        module: await files.exists(join(distribution, release, "src/answer.ts")),
        map: await files.exists(join(distribution, release, "src/answer.ts.map")),
        read: new TextDecoder().decode(await read.load("src/answer.ts.map")),
    }).toEqual({ module: true, map: false, read: map });
});

test("link a running tree's build to the dependencies its closure places alone, never reading a build outside it", async () => {
    // store an app pinning a library it runs with, and a tool it only builds with that the store never pulled
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files, crypto } = environment;
    await using store = await LocalBuildStore.temporary(environment);
    const pin = async (name: string, id: string, isStored: boolean) => {
        const reader = await written(`export const ${name} = 1;`, crypto, {
            id,
            name: `@example/${name}`,
        });

        return {
            kind: "destack" as const,
            package: PackageManifest.package(reader.manifest),
            manifest: isStored
                ? await store.put(reader)
                : await BuildStore.digest(reader.manifest, crypto),
        };
    };
    const library = await pin("library", "package-00000000-0000-7000-8000-00000000a002", true);
    const tool = await pin("tool", "package-00000000-0000-7000-8000-00000000a003", false);
    const app = await written("export const app = 1;", crypto, ANSWER, {
        "@example/library": library,
        "@example/tool": tool,
    });
    const built = {
        kind: "destack" as const,
        package: PackageManifest.package(app.manifest),
        manifest: await store.put(app),
    };

    // materialize it running on Bun over a closure leaving the tool out
    const closure = new Set([built.manifest, library.manifest]);
    const tree = await DependencyTree.materialize(store, [built], store.trees, {
        run: { runtime: "bun", closure },
    });
    const modules = join(present(tree.directories["@example/answer"], "the app"), "..");

    // link the library beside the app but not the tool, and list both roots
    expect({
        linked: (await files.readDirectory(modules)).map((entry) => entry.name).toSorted(),
        roots: (await tree.roots(files)).map((root) => root.slice(root.lastIndexOf("/") + 1)),
    }).toEqual({
        linked: ["answer", "library"],
        roots: [`${built.manifest}.bun`, `${library.manifest}.bun`].toSorted(),
    });
});

test("share the store between opens, each reading what the other writes", async () => {
    // open the store twice at once, as two test processes do
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    await using first = await LocalBuildStore.open(directory, environment);
    await using second = await LocalBuildStore.open(directory, environment);

    // read through each open what the other stored
    const bytes = { first: new TextEncoder().encode("first"), second: MARKER };
    const digests = {
        first: await Digest.of(bytes.first, environment.crypto),
        second: await Digest.of(bytes.second, environment.crypto),
    };
    await first.receive([{ digest: digests.first, bytes: bytes.first }]);
    await second.receive([{ digest: digests.second, bytes: bytes.second }]);
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
    await using building = await LocalBuildStore.open(directory, environment);
    const build = await written("export const answer = 42;\n", environment.crypto);
    const distributed = await build.distributed();
    for (const file of distributed) {
        await building.receive([{ digest: file.digest, bytes: await build.load(file.path) }]);
    }

    // open the store twice elsewhere, which would collect unleased files of an idle store
    for (const attempt of ["marking", "deleting"]) {
        await using opened = await LocalBuildStore.open(directory, environment);
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
        await using store = await LocalBuildStore.open(directory, environment);
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
        await using store = await LocalBuildStore.open(directory, environment);
        rooted = await store.put(await written("export const answer = 42;\n", environment.crypto));
        dropped = await store.put(await written("export const answer = 43;\n", environment.crypto));
        await store.root("answer", new Set([rooted]));
        await files.mkdir(join(store.trees, dropped, "node_modules"), { recursive: true });
        await files.mkdir(join(store.trees, rooted, "node_modules"), { recursive: true });
    }

    // keep the unrooted build through two idle opens within the week
    const contents = async () => {
        await using store = await LocalBuildStore.open(directory, environment);

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

test("collect an unrooted build's layered copy and the linked layers shipping it with the build", async () => {
    // store a rooted build and an unrooted one, each in a linked layer of its own
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const directory = await files.makeTemporaryDirectory("test-stores-");
    const { clock } = simulation;
    let rooted: { readonly build: Digest; readonly layer: string };
    let dropped: { readonly build: Digest; readonly layer: string };
    {
        await using store = await LocalBuildStore.open(directory, environment);
        const put = async (source: string) => {
            const build = await store.put(await written(source, environment.crypto));

            return { build, layer: await store.writeLinkedLayer([build], []) };
        };
        rooted = await put("export const answer = 42;\n");
        dropped = await put("export const answer = 43;\n");
        await store.root("answer", new Set([rooted.build]));
    }

    // read which builds and layers the store keeps on an idle open
    const contents = async () => {
        await using store = await LocalBuildStore.open(directory, environment);
        const layered = (build: Digest) => files.exists(join(directory, "layered", build));

        return {
            rooted: [await layered(rooted.build), await files.exists(rooted.layer)],
            dropped: [await layered(dropped.build), await files.exists(dropped.layer)],
        };
    };
    const kept = await contents();

    // collect the unrooted build's copy and layer on the first idle open past the week
    await simulation.jump(clock.now() + WEEK_MILLISECONDS);
    expect({ kept, collected: await contents() }).toEqual({
        kept: { rooted: [true, true], dropped: [true, true] },
        collected: { rooted: [true, true], dropped: [false, false] },
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
        await using store = await LocalBuildStore.open(directory, environment);
        used = await store.put(await written("export const answer = 44;\n", environment.crypto));
    }
    await simulation.jump(clock.now() + DAY_MILLISECONDS * 6);
    {
        await using store = await LocalBuildStore.open(directory, environment);
        await store.contents(used);
    }

    // read whether two idle opens a day apart keep it
    const contains = async () => {
        await using store = await LocalBuildStore.open(directory, environment);

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

test("renew a build a world's view used once the view closes, retaining it a week past that use", async () => {
    // store an unrooted build, and use it six days later through a view another node opens
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const directory = await environment.files.makeTemporaryDirectory("test-stores-");
    const { clock } = simulation;
    let used: Digest;
    {
        await using store = await LocalBuildStore.open(directory, environment);
        used = await store.put(await written("export const answer = 45;\n", environment.crypto));
    }
    await simulation.jump(clock.now() + DAY_MILLISECONDS * 6);
    {
        const host = await LocalBuildStore.open(directory, environment);
        await using view = await host.view(simulation.environment("world"));
        await view.contents(used);
    }

    // read whether an idle open keeps it past the week after storing it, then a week after its use
    const contains = async () => {
        await using store = await LocalBuildStore.open(directory, environment);

        return await store.contains(used);
    };
    await simulation.jump(clock.now() + DAY_MILLISECONDS * 2);
    const kept = await contains();
    await simulation.jump(clock.now() + WEEK_MILLISECONDS);
    const dropped = await contains();
    expect({ kept, dropped }).toEqual({ kept: true, dropped: false });
});

test("refuse an empty cache directory the environment sets for the person's store", async () => {
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
    expect(await refusal(LocalBuildStore.user("test", environment))).toEqual([
        "BAD_REQUEST",
        "CACHE_DIRECTORY_INVALID",
        "DESTACK_CACHE_DIRECTORY is empty",
    ]);
});
