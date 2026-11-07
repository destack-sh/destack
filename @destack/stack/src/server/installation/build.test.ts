import { BuildStore } from "@destack/build/store";
import { TestBuildStore } from "@destack/build/test";
import { testSimulation } from "@destack/object/test";
import { Package, type PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { Commit, type Digest } from "@destack/schema";
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
    const builds = new SpaceBuildStore(space, new BuildCache(pulled, ["bun"]));
    await builds.pull(recording, APPLICATION.id, applicationManifest);

    // locate each build once, storing the release whole and only the Bun outputs of the builds its closure reaches
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
            [true, true],
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
    const builds = new SpaceBuildStore(space, new BuildCache(pulled, ["bun"]));
    await builds.pull(
        { locate: () => Promise.reject(new Error("no build is published")) },
        APPLICATION.id,
        applicationManifest,
    );

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
