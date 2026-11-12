import { BuildStore } from "@destack/build/store";
import { TestBuildStore } from "@destack/build/test";
import { testSimulation } from "@destack/object/test";
import { PackageVersion } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { type Digest, schema } from "@destack/schema";
import { Lock } from "@destack/package/manifest";
import { expect, test, vi } from "@destack/test";
import { fixtureOutputs, TestRegistry, workload } from "../../test/build.ts";
import { BuildCache, SpaceBuildStore } from "./build.ts";

/** The npm release the library's build depends on. */
const TINY = { name: "tiny", version: "1.0.0", integrity: "sha512-dGlueQ==" };

/** The library the application depends on. */
const LIBRARY = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b001",
    name: "@example/library",
    version: "2026.9.0",
});

/** The second library the application depends on, importing the same npm unit as the first. */
const OTHER_LIBRARY = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b003",
    name: "@example/other-library",
    version: "2026.9.0",
});

/** The application whose build is pulled. */
const APPLICATION = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b002",
    name: "@example/application",
    version: "2026.9.0",
});

/** The builtin the machine links. */
const BUILTIN = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b004",
    name: "@example/builtin",
    version: "2026.11.0",
});

/** The npm unit of the tiny release, with a Bun output and its types. */
function tinyUnit(crypto: Parameters<typeof MemoryBuild.write>[2]) {
    return MemoryBuild.write(
        new Map([
            ["package.json", new TextEncoder().encode(JSON.stringify(TINY))],
            ["bun/index.js", new TextEncoder().encode("export const tiny = 1;\n")],
            ["bun/index.d.ts", new TextEncoder().encode("export declare const tiny: number;\n")],
        ]),
        {
            role: "app",
            package: TINY,
            outputs: fixtureOutputs({
                bun: { runtime: "bun", emit: true, workloads: {}, views: {} },
            }),
        },
        crypto,
    );
}

/** The outputs of a library importing the tiny unit. */
const IMPORTING_TINY = {
    outputs: fixtureOutputs({
        bun: {
            runtime: "bun",
            emit: true,
            workloads: {},
            views: {},
            exports: { ".": { path: "bun/index.js", imports: ["tiny"] } },
        },
    }),
    requires: { tiny: TINY.version },
};

test("pull the Bun outputs of a release lock's runtime closure", async () => {
    // publish an application
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using registered = await TestBuildStore.create(environment);
    const registry = new TestRegistry(registered, APPLICATION.id);
    const unit = await tinyUnit(crypto);
    const library = await MemoryBuild.of(LIBRARY, [], crypto, IMPORTING_TINY);
    const application = await MemoryBuild.of(APPLICATION, [], crypto, {
        outputs: fixtureOutputs({
            bun: {
                runtime: "bun",
                emit: true,
                workloads: { serve: { ...workload, entrypoint: "./serve" } },
                views: {},
                exports: { "./serve": { path: "bun/serve.js", imports: [LIBRARY.name] } },
            },
        }),
        requires: { [LIBRARY.name]: LIBRARY.version },
    });
    const lock = await registry.release(application, [library, unit]);
    const applicationManifest = await BuildStore.digest(application.manifest, crypto);
    const libraryManifest = await BuildStore.digest(library.manifest, crypto);
    const unitManifest = await BuildStore.digest(unit.manifest, crypto);

    // pull the application's lock into an empty store
    await using space = await TestBuildStore.create(environment);
    await using pulled = await TestBuildStore.create(environment);
    const builds = new SpaceBuildStore(space, new BuildCache(pulled, ["bun"], new Map()), registry);
    await builds.pull(APPLICATION.id, lock);

    // store the lock, each build's manifest and the Bun outputs, without the types
    const store = builds.cache.store;
    const state = async (manifest: Digest) => [
        await store.contains(manifest),
        await store.containsOutput(manifest, "bun"),
    ];
    const typed = (await store.contents(unitManifest)).file("bun/index.d.ts");
    expect({
        states: [
            await state(applicationManifest),
            await state(libraryManifest),
            await state(unitManifest),
        ],
        isLocked: await store.contains(lock),
        isTypeStored: (await store.has([(await typed).digest]))[0],
    }).toEqual({
        states: [
            [true, true],
            [true, true],
            [false, true],
        ],
        isLocked: true,
        isTypeStored: false,
    });
});

test("pull a space's own lock whose libraries share an npm unit, reading each build once", async () => {
    // lock an application with two libraries importing one npm unit
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using space = await TestBuildStore.create(environment);
    const unitManifest = await space.put(await tinyUnit(crypto));
    const libraryManifest = await space.put(
        await MemoryBuild.of(LIBRARY, [], crypto, IMPORTING_TINY),
    );
    const otherManifest = await space.put(
        await MemoryBuild.of(OTHER_LIBRARY, [], crypto, IMPORTING_TINY),
    );
    const applicationManifest = await space.put(
        await MemoryBuild.of(APPLICATION, [], crypto, {
            requires: {
                [LIBRARY.name]: LIBRARY.version,
                [OTHER_LIBRARY.name]: OTHER_LIBRARY.version,
            },
        }),
    );
    const locked = {
        [LIBRARY.name]: libraryManifest,
        [OTHER_LIBRARY.name]: otherManifest,
        tiny: unitManifest,
    };
    const lock = await space.putLock(
        await Lock.hoisted(applicationManifest, locked, (digest) => space.get(digest)),
    );

    // pull the application's lock
    await using pulled = await TestBuildStore.create(environment);
    const reads = vi.spyOn(space, "read");
    const builds = new SpaceBuildStore(space, new BuildCache(pulled, ["bun"], new Map()), {
        pull: () => Promise.reject(new Error("no lock is published")),
        store: () => Promise.reject(new Error("no build is published")),
    });
    await builds.pull(APPLICATION.id, lock);

    // read the application, each library and the shared unit once
    const counted = new Map<string, number>();
    for (const [objects] of reads.mock.calls) {
        for (const object of objects.filter((each) => each.kind === "manifest")) {
            counted.set(object.digest, (counted.get(object.digest) ?? 0) + 1);
        }
    }
    expect(Object.fromEntries(counted)).toEqual({
        [String(applicationManifest)]: 1,
        [String(libraryManifest)]: 1,
        [String(otherManifest)]: 1,
        [String(unitManifest)]: 1,
    });
});

test("read a builtin at its linked lock's root whatever build is asked", async () => {
    // ship the builtin's lock with the machine
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using registered = await TestBuildStore.create(environment);
    const registry = new TestRegistry(registered, APPLICATION.id);
    await using pulled = await TestBuildStore.create(environment);
    const linked = await pulled.put(await MemoryBuild.of(BUILTIN, [], crypto));
    const linkedLock = await pulled.putLock(Lock.of(linked));
    const application = await MemoryBuild.of(APPLICATION, [], crypto, {
        outputs: fixtureOutputs({
            bun: {
                runtime: "bun",
                emit: true,
                workloads: { serve: { ...workload, entrypoint: "./serve" } },
                views: {},
                exports: { "./serve": { path: "bun/serve.js", imports: [BUILTIN.name] } },
            },
        }),
    });
    const lock = await registry.release(application);

    // pull the application, then read the builtin at a build its publisher compiled against
    await using space = await TestBuildStore.create(environment);
    const cache = new BuildCache(
        pulled,
        ["bun"],
        new Map([[BUILTIN.id, { package: BUILTIN, lock: linkedLock }]]),
    );
    const builds = new SpaceBuildStore(space, cache, registry);
    await builds.pull(APPLICATION.id, lock);
    const compiled = await BuildStore.digest(
        (await MemoryBuild.of({ ...BUILTIN, version: "2026.10.0" }, [], crypto)).manifest,
        crypto,
    );
    const read = await builds.read(BUILTIN.id, compiled);

    // read the linked build in place of the compiled one
    expect(await BuildStore.digest(read.manifest, crypto)).toEqual(linked);
});

test("retain a space's builds through a collection until the machine prunes the space", async () => {
    // pull a library's build into the machine's cache, rooted by one space and leased by none
    const environment = testSimulation().environment("test");
    await using pulled = await TestBuildStore.create(environment);
    const manifest = await pulled.put(await MemoryBuild.of(LIBRARY, [], environment.crypto));
    const cache = new BuildCache(pulled, ["bun"], new Map());
    const space = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000b5");
    await cache.root(space, [manifest]);

    // end the pull's lease and collect, then prune the space and collect again
    await pulled.evict(0);
    const isRetained = await pulled.contains(manifest);
    await cache.prune([]);
    await pulled.blobs.collect(0);

    // keep the build while the space roots it, and drop it with the space
    expect({ isRetained, isPruned: !(await pulled.contains(manifest)) }).toEqual({
        isRetained: true,
        isPruned: true,
    });
});
