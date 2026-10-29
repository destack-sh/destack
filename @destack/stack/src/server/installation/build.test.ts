import { PackageStore } from "@destack/build/store";
import { TestPackageStore } from "@destack/build/test";
import { testSimulation } from "@destack/object/test";
import { Package, type PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { Commit, type Digest, present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { fixtureOutputs, TestRegistry, workload } from "../../test/build.ts";
import { BuildCache, BuildStore } from "./build.ts";

/** The npm release the library's build depends on. */
const TINY = { name: "tiny", version: "1.0.0", integrity: "sha512-dGlueQ==" };

/** The library the application depends on. */
const LIBRARY = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000b001",
    name: "@example/library",
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

test("pull a release with the runtime builds of its runtime closure, finding each at its package's registry and an npm unit at its dependent's", async () => {
    // publish a library importing an npm unit, and an application whose workload imports the library
    const environment = testSimulation().environment("test");
    const { crypto } = environment;
    await using registered = await TestPackageStore.create(environment);
    const registry = new TestRegistry(registered);
    const packageJson = new TextEncoder().encode(JSON.stringify(TINY));
    const unit = await MemoryBuild.write(
        new Map([
            ["package.json", packageJson],
            ["bun/index.js", new TextEncoder().encode("export const tiny = 1;\n")],
            ["bun/index.d.ts", new TextEncoder().encode("export declare const tiny: number;\n")],
        ]),
        {
            package: TINY,
            outputs: fixtureOutputs({
                bun: { runtime: "bun", emit: true, workloads: {}, views: {} },
            }),
        },
        crypto,
    );
    const unitManifest = await PackageStore.digest(unit.manifest, crypto);
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
        referrer: (packageId: PackageId, manifest: Digest, path: string) => {
            located.push([packageId, manifest]);

            return registry.referrer(packageId, manifest, path);
        },
    };
    await using space = await TestPackageStore.create(environment);
    await using pulled = await TestPackageStore.create(environment);
    const builds = new BuildStore(space, new BuildCache(pulled, ["bun"]));
    await builds.pull(recording, APPLICATION.id, applicationManifest);

    // read the runtime builds the store attached, each naming its dependencies' runtime builds
    const runtimeOf = async (manifest: Digest) =>
        present(await builds.cache.store.referrer(manifest, "runtime/bun"), "a runtime build")
            .digest;
    const dependencyOf = async (manifest: Digest, name: string) =>
        present(
            Object.values((await builds.cache.store.get(manifest)).outputs)[0]?.dependencies[name],
            name,
        ).manifest;
    const applicationRuntime = await runtimeOf(applicationManifest);
    const libraryRuntime = await dependencyOf(applicationRuntime, LIBRARY.name);
    const unitRuntime = await dependencyOf(libraryRuntime, TINY.name);

    // locate the release, its runtime build and the runtime closure, storing the release whole and the closure's runtime builds alone
    const stored = await Promise.all(
        [
            applicationManifest,
            applicationRuntime,
            libraryRuntime,
            unitRuntime,
            libraryManifest,
            unitManifest,
        ].map((manifest) => builds.cache.store.contains(manifest)),
    );
    expect({ located, stored }).toEqual({
        located: [
            [APPLICATION.id, applicationManifest],
            [APPLICATION.id, applicationManifest],
            [APPLICATION.id, applicationRuntime],
            [LIBRARY.id, libraryRuntime],
            [LIBRARY.id, unitRuntime],
        ],
        stored: [true, true, true, true, false, false],
    });
});
