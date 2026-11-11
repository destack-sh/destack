import { BuildStore } from "@destack/build/store";
import { TestBuildStore } from "@destack/build/test";
import { testSimulation } from "@destack/object/test";
import { Package, type PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { Commit, type Digest, schema } from "@destack/schema";
import { expect, test, vi } from "@destack/test";
import { fixtureOutputs, TestRegistry, workload } from "../../test/build.ts";
import { BuildCache, SpaceBuildStore } from "./build.ts";

/** The npm release the library's build depends on. */
const TINY = { name: "tiny", version: "1.0.0", integrity: "sha512-dGlueQ==" };

/** The library the application depends on. */
const LIBRARY = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b001",
    name: "@example/library",
    version: "2026.9.0",
});

/** The second library the application depends on, pinning the same npm unit as the first. */
const OTHER_LIBRARY = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b003",
    name: "@example/other-library",
    version: "2026.9.0",
});

/** The application whose build is pulled. */
const APPLICATION = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b002",
    name: "@example/application",
    version: "2026.9.0",
});

/** The builtin the machine links. */
const BUILTIN = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b004",
    name: "@example/builtin",
    version: "2026.11.0",
});

/** The commit the fixture builds compiled. */
const COMMIT = Commit.parse("a".repeat(40));

test("pull the Bun outputs of a release's runtime closure, locating each build once at its package's registry and an npm unit at its dependent's", async () => {
    // publish a library importing an npm unit, and an application whose workload imports the library
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using registered = await TestBuildStore.create(environment);
    const registry = new TestRegistry(registered, APPLICATION.id);
    const packageJson = new TextEncoder().encode(JSON.stringify(TINY));
    const unit = await MemoryBuild.write(
        new Map([
            ["package.json", packageJson],
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
    const unitManifest = await BuildStore.digest(unit.manifest, crypto);
    const library = await MemoryBuild.of(LIBRARY, [], crypto, {
        commit: COMMIT,
        outputs: fixtureOutputs({
            bun: {
                runtime: "bun",
                emit: true,
                workloads: {},
                views: {},
                exports: { ".": { path: "bun/index.js", imports: ["tiny"] } },
                dependencies: {
                    tiny: {
                        kind: "npm",
                        package: { name: TINY.name, version: TINY.version },
                        registry: "https://registry.npmjs.org",
                        integrity: TINY.integrity,
                        manifest: unitManifest,
                    },
                },
            },
        }),
    });
    const libraryManifest = await registry.release(library, [unit]);
    const application = await MemoryBuild.of(APPLICATION, [], crypto, {
        commit: COMMIT,
        outputs: fixtureOutputs({
            bun: {
                runtime: "bun",
                emit: true,
                workloads: { serve: { ...workload, entrypoint: "./serve" } },
                views: {},
                exports: { "./serve": { path: "bun/serve.js", imports: [LIBRARY.name] } },
                dependencies: {
                    [LIBRARY.name]: {
                        kind: "destack",
                        package: LIBRARY,
                        manifest: libraryManifest,
                    },
                },
            },
        }),
    });
    const applicationManifest = await registry.release(application);

    // pull the application into an empty store, recording where each build is found
    const located: [PackageId, Digest][] = [];
    const recording = {
        locate: (packageId: PackageId, manifest: Digest) => {
            located.push([packageId, manifest]);

            return registry.locate(packageId, manifest);
        },
    };
    await using space = await TestBuildStore.create(environment);
    await using pulled = await TestBuildStore.create(environment);
    const builds = new SpaceBuildStore(
        space,
        new BuildCache(pulled, ["bun"], new Map()),
        recording,
    );
    await builds.pull(APPLICATION.id, applicationManifest);

    // locate each build once, storing only the Bun outputs of the release and the builds its closure reaches
    const store = builds.cache.store;
    const state = async (manifest: Digest) => [
        await store.contains(manifest),
        await store.containsOutput(manifest, "bun"),
    ];
    const typed = (await store.contents(unitManifest)).file("bun/index.d.ts");
    expect({
        located,
        states: [
            await state(applicationManifest),
            await state(libraryManifest),
            await state(unitManifest),
        ],
        isTypeStored: await store.has((await typed).digest),
    }).toEqual({
        located: [
            [APPLICATION.id, applicationManifest],
            [LIBRARY.id, libraryManifest],
            [LIBRARY.id, unitManifest],
        ],
        states: [
            [false, true],
            [false, true],
            [false, true],
        ],
        isTypeStored: false,
    });
});

test("pull a space's own build whose dependencies share a build, reading each build once", async () => {
    // build an npm unit, two libraries pinning it, and an application pinning both, all in the space
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using space = await TestBuildStore.create(environment);
    const unit = await MemoryBuild.write(
        new Map([["package.json", new TextEncoder().encode(JSON.stringify(TINY))]]),
        {
            role: "app",
            package: TINY,
            outputs: fixtureOutputs({
                bun: { runtime: "bun", emit: true, workloads: {}, views: {} },
            }),
        },
        crypto,
    );
    const unitManifest = await space.put(unit);
    const tiny = {
        kind: "npm" as const,
        package: { name: TINY.name, version: TINY.version },
        registry: "https://registry.npmjs.org",
        integrity: TINY.integrity,
        manifest: unitManifest,
    };
    const pinningTiny = {
        commit: COMMIT,
        outputs: fixtureOutputs({
            bun: { runtime: "bun", emit: true, workloads: {}, views: {}, dependencies: { tiny } },
        }),
    } as const;
    const libraryManifest = await space.put(await MemoryBuild.of(LIBRARY, [], crypto, pinningTiny));
    const otherManifest = await space.put(
        await MemoryBuild.of(OTHER_LIBRARY, [], crypto, pinningTiny),
    );
    const applicationManifest = await space.put(
        await MemoryBuild.of(APPLICATION, [], crypto, {
            commit: COMMIT,
            outputs: fixtureOutputs({
                bun: {
                    runtime: "bun",
                    emit: true,
                    workloads: {},
                    views: {},
                    dependencies: {
                        [LIBRARY.name]: {
                            kind: "destack",
                            package: LIBRARY,
                            manifest: libraryManifest,
                        },
                        [OTHER_LIBRARY.name]: {
                            kind: "destack",
                            package: OTHER_LIBRARY,
                            manifest: otherManifest,
                        },
                    },
                },
            }),
        }),
    );

    // pull the application, counting the space's reads of each build
    await using pulled = await TestBuildStore.create(environment);
    const reads = vi.spyOn(space, "contents");
    const builds = new SpaceBuildStore(space, new BuildCache(pulled, ["bun"], new Map()), {
        locate: () => Promise.reject(new Error("no build is published")),
    });
    await builds.pull(APPLICATION.id, applicationManifest);

    // read the application, each library and the shared unit once
    const counted = new Map<string, number>();
    for (const [manifest] of reads.mock.calls) {
        counted.set(String(manifest), (counted.get(String(manifest)) ?? 0) + 1);
    }
    expect(Object.fromEntries(counted)).toEqual({
        [String(applicationManifest)]: 1,
        [String(libraryManifest)]: 1,
        [String(otherManifest)]: 1,
        [String(unitManifest)]: 1,
    });
});

test("load a builtin's linked build whatever build a release recorded, pulling none of it", async () => {
    // ship the builtin with the machine, and publish an application recording it by its release
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using registered = await TestBuildStore.create(environment);
    const registry = new TestRegistry(registered, APPLICATION.id);
    await using pulled = await TestBuildStore.create(environment);
    const linked = await pulled.put(await MemoryBuild.of(BUILTIN, [], crypto, { commit: COMMIT }));
    const application = await MemoryBuild.of(APPLICATION, [], crypto, {
        commit: COMMIT,
        outputs: fixtureOutputs({
            bun: {
                runtime: "bun",
                emit: true,
                workloads: { serve: { ...workload, entrypoint: "./serve" } },
                views: {},
                exports: { "./serve": { path: "bun/serve.js", imports: [BUILTIN.name] } },
                dependencies: { [BUILTIN.name]: { kind: "builtin", package: BUILTIN } },
            },
        }),
    });
    const applicationManifest = await registry.release(application);

    // pull the application, then read the builtin at a build its publisher compiled against
    const located: PackageId[] = [];
    const recording = {
        locate: (packageId: PackageId, manifest: Digest) => {
            located.push(packageId);

            return registry.locate(packageId, manifest);
        },
    };
    await using space = await TestBuildStore.create(environment);
    const cache = new BuildCache(
        pulled,
        ["bun"],
        new Map([[BUILTIN.id, { package: BUILTIN, manifest: linked }]]),
    );
    const builds = new SpaceBuildStore(space, cache, recording);
    await builds.pull(APPLICATION.id, applicationManifest);
    const compiled = await BuildStore.digest(
        (await MemoryBuild.of({ ...BUILTIN, version: "2026.10.0" }, [], crypto)).manifest,
        crypto,
    );
    const read = await builds.read(BUILTIN.id, compiled);

    // locate the application alone, and read the linked build
    expect({
        located,
        read: await BuildStore.digest(read.manifest, crypto),
    }).toEqual({ located: [APPLICATION.id], read: linked });
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
