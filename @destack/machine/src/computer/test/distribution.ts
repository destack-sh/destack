import { fixtureGraph } from "@destack/build/test";
import type { Environment } from "@destack/environment";
import { fileURLToPath } from "node:url";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition, ReleaseRegistry } from "@destack/account/server";
import { type LocalBuildStore, Workspace } from "@destack/build";
import { BuildStore } from "@destack/build/store";
import { BuildLayer } from "@destack/package/manifest";
import { testStore } from "@destack/build/test";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import { InstallationMount, PackageMount } from "@destack/object";
import {
    DistributionManifest,
    DistributionPlatform,
    type PackageId,
    PackageName,
    PackageRelease,
} from "@destack/package";
import { BuildReader, BuildImage, PackageManifest } from "@destack/package/manifest";
import { Registry } from "@destack/registry/client";
import { LATEST_TAG } from "@destack/registry/object";
import registryPackage from "@destack/registry/package";
import { Digest, found, present, schema } from "@destack/schema";
import { inject, onTestFinished } from "@destack/test";
import { testHost } from "@destack/test/host";
import { COMPUTER_RUNTIMES } from "../computer.ts";
import type { Machine } from "../../machine/index.ts";
import { MachineError } from "../../error/index.ts";

/** The run's provided values the worlds read. */
declare module "vitest" {
    /** The values a run's setup provides to its tests. */
    export interface ProvidedContext {
        /** The manifest digests of the run's builds, by name, absent without a setup. */
        built?: Readonly<Record<string, string>>;
        /** The manifest digests of the packages only scenarios install, by name. */
        scenarios?: Readonly<Record<string, string>>;
        /** Where computers' state directories move after their tests, absent without a setup. */
        trash?: string;
    }
}

/** The variable the run's setup writes templates with: its images' digests by name. */
export const BUILDS_VARIABLE = "DESTACK_MACHINE_BUILDS";

/** The platform packages a test distribution ships, built from the workspace as releases are. */
export const PLATFORM_PACKAGES = [
    "@destack/vault",
    "@destack/message",
    "@destack/notification",
    "@destack/connection",
    "@destack/social",
    "@destack/observability",
    "@destack/forge",
    "@destack/home",
] as const;

/** The package the test computers run, whose release a distribution ships as their own. */
export const RUNTIME_PACKAGE = "@destack/machine";

/** The program people's computers run, which a distribution ships beside the runtime. */
export const DESKTOP_PACKAGE = "@destack/desktop";

/** The fixture package declaring the stacks spaces are born with, as the platform's does. */
export const STACK_FIXTURE = "stack";

/** The fixture start page the stack fixture installs in every person's home. */
export const START_FIXTURE = "start";

/** How long a machine's directory may take to list a new organization: 30 s, a few syncs. */
const DIRECTORY_MILLISECONDS = 30_000;

/** The commit a test publish releases at, the all-zero SHA-1. */
const TEST_COMMIT = "0".repeat(40);

/** How long a residency home may take to serve its forge: 30 s, as a fresh home deploys it. */
const FORGE_MILLISECONDS = 30_000;

/** The publisher of a scope's packages: a person managing or publishing for its account. */
export type TestPublisher = Pick<Machine, "fetch">;

/** What a test computer ships: the platform's stacks, what they install, and other builds. */
export class TestDistribution implements ReleaseRegistry, AsyncDisposable {
    /** The stacks spaces are born with by kind. */
    readonly stacks: PlatformDefinition["stacks"];
    /** The stack each residency's home runs. */
    readonly home: StackRelease;
    /** The root builds of the shipped releases: the platform's units, stacks and given builds. */
    readonly builds: readonly BuildReader[];
    /** The manifest of the releases the distribution ships, each with its role. */
    readonly manifest: DistributionManifest;
    /** The platform the manifest has the images of, the host's. */
    readonly platform: DistributionPlatform;
    /** The builds with what they run, as a machine ships them in place. */
    readonly layer: BuildLayer;
    /** The run's build store the server builds read from, open until the distribution closes. */
    readonly #store: LocalBuildStore;

    /** The layers this process writes or wrote, by their releases, each written once. */
    static readonly #distributed = new Map<string, Promise<string>>();

    /** Take the stacks, the builds, the shipped releases, their platform, layer and store. */
    private constructor(
        stacks: PlatformDefinition["stacks"],
        home: StackRelease,
        builds: readonly BuildReader[],
        manifest: DistributionManifest,
        platform: DistributionPlatform,
        layer: BuildLayer,
        store: LocalBuildStore,
    ) {
        // set the stacks, the residency homes', the builds, the shipped releases, their layer and their store
        this.stacks = stacks;
        this.home = home;
        this.builds = builds;
        this.manifest = manifest;
        this.platform = platform;
        this.layer = layer;
        this.#store = store;
    }

    /** Ship the run's images of the platform and its stacks, and the builds a test adds. */
    static async build(
        host: Environment<"files" | "process" | "host">,
        given: readonly BuildReader[] = [],
    ): Promise<TestDistribution> {
        // read the run's images in its store
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await testStore(host));
        const built = TestDistribution.built();
        const imageOf = (name: string) => present(built[name], `the run's image of ${name}`);
        const read = async (name: string) => store.root(imageOf(name));

        // select the stacks spaces are born with
        const stack = await read(STACK_FIXTURE);
        const stacks = {
            home: TestDistribution.released(stack, "HOME_STACK"),
            space: TestDistribution.released(stack, "SPACE_STACK"),
        };
        const residencyHome = TestDistribution.released(stack, "RESIDENCY_STACK");

        // ship the computers' own release, the stacks, the packages they install, and the builds the test adds
        const extra = await Promise.all(
            given.map((build) => TestDistribution.#imageAt(build, store)),
        );
        const shipped = await TestDistribution.shipped(store, built, extra);
        const target = DistributionPlatform.of(host.host);
        const releases = await Promise.all(
            shipped.map(async (image) => {
                const root = await store.get((await store.image(image)).root);

                return {
                    package: PackageManifest.package(root),
                    role: root.role,
                    images: { [target]: image },
                };
            }),
        );
        const layer = await TestDistribution.#distribute(shipped, store);

        // read the root of every shipped release, which the distribution opens by version
        const builds = await Promise.all(shipped.map((image) => store.root(image)));

        // hand the store to the distribution
        opened.move();

        return new TestDistribution(
            stacks,
            residencyHome,
            builds,
            { releases },
            target,
            layer,
            store,
        );
    }

    /** Make another image of a build at another version, with the builds it runs with. */
    static async release(store: LocalBuildStore, image: Digest, version: string): Promise<Digest> {
        // stamp the root build's manifest with the version, keeping its files
        const previous = await store.image(image);
        const root = await store.contents(previous.root);
        const manifest = {
            ...root.manifest,
            package: { ...PackageManifest.package(root.manifest), version },
        };
        const reader = new BuildReader(manifest, root.stream, store.environment.crypto);
        const released = await TestDistribution.#putIndexed(store, reader);

        // make an image of it in the root's place with the root's links
        const { [previous.root]: links, ...builds } = previous.builds;
        const linked = present(links, `the links of build ${previous.root}`);

        return await store.putImage(BuildImage.of(released, { ...builds, [released]: linked }));
    }

    /** Open a fixture package's build in the run's store for a lifetime, the test's by default. */
    static async fixture(
        fixture: string,
        environment?: Environment<"files" | "process" | "host">,
        defer: (cleanup: () => Promise<void>) => void = onTestFinished,
    ): Promise<BuildReader> {
        // open the store for
        const image = TestDistribution.imageOf(fixture);
        const store = await testStore(environment);
        defer(() => store[Symbol.asyncDispose]());

        return await store.root(image);
    }

    /** Read the digests of the run's images by name: the platform, the stacks and the fixtures. */
    static built(): Readonly<Record<string, Digest>> {
        const written = testHost().process.variables[BUILDS_VARIABLE];
        const built: unknown =
            written === undefined ? (inject("built") ?? {}) : JSON.parse(written);

        return schema.record(schema.string(), Digest).parse(built);
    }

    /** Read the images of the packages only scenarios install, which no distribution ships. */
    static scenarios(): Readonly<Record<string, Digest>> {
        return schema.record(schema.string(), Digest).parse(inject("scenarios") ?? {});
    }

    /** Find the run's image rooted at a given build, refusing a build the run did not image. */
    static async #imageAt(build: BuildReader, store: LocalBuildStore): Promise<Digest> {
        // compare the build with each run image's root
        const root = await BuildStore.digest(build.manifest, store.environment.crypto);
        const images = [
            ...Object.values(TestDistribution.built()),
            ...Object.values(TestDistribution.scenarios()),
        ];
        for (const image of images) {
            if ((await store.image(image)).root === root) {
                return image;
            }
        }
        throw new TypeError(`the run made no image of build '${root}'`);
    }

    /** Read the run's image of a fixture package. */
    static imageOf(fixture: string): Digest {
        return present(
            TestDistribution.built()[fixture] ?? TestDistribution.scenarios()[fixture],
            `the run's image of ${fixture}`,
        );
    }

    /** Put a build with its fixture graph attached once, as a builder images what it ships. */
    static async #putIndexed(store: LocalBuildStore, reader: BuildReader): Promise<Digest> {
        const digest = await store.put(reader);
        if (!(await store.isIndexed(digest))) {
            await store.attachGraph(digest, await fixtureGraph(reader));
        }

        return digest;
    }

    /** Store a build in an image with the builds it runs with, answering the image's digest. */
    async image(build: BuildReader, builds: readonly BuildReader[] = []): Promise<Digest> {
        // store each build by its package
        const digests: Record<string, Digest> = {};
        for (const each of builds) {
            digests[each.manifest.package.name] = await TestDistribution.#putIndexed(
                this.#store,
                each,
            );
        }
        const root = await TestDistribution.#putIndexed(this.#store, build);

        // make an image of them hoisted
        const image = await BuildImage.hoisted(root, digests, (digest) => this.#store.get(digest));

        return await this.#store.putImage(image);
    }

    /** Publish an image in the run's store into its root package's forge as its publisher. */
    async publish(image: Digest, publisher: TestPublisher, machine: Machine): Promise<Digest> {
        // read the image's root build
        const { directory, environment } = machine;
        const build = await this.#store.root(image);

        // wait until the machine's replica of the directory has the account of the package's scope
        const { name } = PackageManifest.package(build.manifest);
        const parsed = PackageName.split(name);
        const isListed = async () => (await directory.account(parsed.handle)) !== undefined;
        const listing = environment.clock.timeout(DIRECTORY_MILLISECONDS);
        await machine.database.log.waitUntil(isListed, listing);

        // find the account and the home of its residency
        const accountId = present(await directory.account(parsed.handle), `@${parsed.handle}`);
        const located = await (await machine.accounts()).residency.locate({ account: accountId });
        const home = schema.identifier("space").parse(located);

        // wait until the home serves its forge
        const space = await machine.space(home);
        await space.waitServed(FORGE_INSTALLATION, environment.clock.timeout(FORGE_MILLISECONDS));

        // call the forge below its installation in the home as the publisher, bound to the home
        const fetch = publisher.fetch(registryPackage.id, { partition: home });
        const endpoint = await directory.locate(home, environment.clock.now());
        const origin = InstallationMount.url(endpoint.url, home, FORGE_INSTALLATION);
        const url = `${origin}${PackageMount.path(registryPackage.id)}`;
        const registry = new Registry(async () => ({ url, space: home, fetch }), environment);

        // push its image and publish it at the test commit
        await registry.release(this.#store, [image], accountId, LATEST_TAG, {
            commit: TEST_COMMIT,
        });

        return image;
    }

    /** List the run's images a distribution ships: the platform, its stacks, builtins and more. */
    static async shipped(
        store: Pick<LocalBuildStore, "get" | "image">,
        built: Readonly<Record<string, Digest>>,
        given: readonly Digest[],
    ): Promise<Digest[]> {
        // take the named packages' images and the given ones
        const imageOf = (name: string) => present(built[name], `the run's image of ${name}`);
        const named = [
            RUNTIME_PACKAGE,
            ...PLATFORM_PACKAGES,
            DESKTOP_PACKAGE,
            STACK_FIXTURE,
            START_FIXTURE,
        ];
        const images = [...named.map((name) => imageOf(name)), ...given];

        // ship every builtin's and library's image too, as a platform build releases each unit
        for (const image of Object.values(built)) {
            const root = await store.get((await store.image(image)).root);
            if (root.role !== "app") {
                images.push(image);
            }
        }

        return [...new Set(images)];
    }

    /** Write a layer of release images once per process as a distribution does, and open it. */
    static async #distribute(
        releases: readonly Digest[],
        store: LocalBuildStore,
    ): Promise<BuildLayer> {
        // write it once per process on the host's store, one long operation of the world whether written before or not
        const key = JSON.stringify(releases);
        const writing = TestDistribution.#distributed.get(key) ?? TestDistribution.#write(releases);
        TestDistribution.#distributed.set(key, writing);
        const directory = await store.environment.scheduler.run(() => writing, { isLong: true });

        return await BuildLayer.open(directory, store.environment);
    }

    /** Write a linked layer of release images in the host's store, returning its directory. */
    static async #write(releases: readonly Digest[]): Promise<string> {
        await using store = await testStore();

        return await store.writeLinkedLayer(releases, COMPUTER_RUNTIMES);
    }

    /** The releases the distribution ships, which a scope runs of a package it installs none of. */
    get distribution(): readonly PackageRelease[] {
        return DistributionManifest.releases(this.manifest, this.platform);
    }

    /** Open a shipped release of a package by its version, refusing one the distribution lacks. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        const shipped = this.builds.find(({ manifest }) => {
            const built = manifest.package;

            return "id" in built && built.id === packageId && built.version === release;
        });
        if (shipped === undefined) {
            throw MachineError.of(
                "PACKAGE_NOT_FOUND",
                `release '${release}' of this package on the test computer not found`,
            );
        }

        return shipped;
    }

    /** Build a platform package from the workspace, found through machine's dependency on it. */
    static platform(
        name: (typeof PLATFORM_PACKAGES)[number],
        workspace: Workspace,
    ): ReturnType<typeof TestDistribution.compile> {
        const directory = fileURLToPath(new URL(`../../../node_modules/${name}/`, import.meta.url));

        return TestDistribution.compile(workspace, directory);
    }

    /** Build the desktop from its workspace directory, as production's distribution ships it. */
    static desktop(workspace: Workspace): ReturnType<typeof TestDistribution.compile> {
        const directory = fileURLToPath(new URL("../../../../desktop/", import.meta.url));

        return TestDistribution.compile(workspace, directory);
    }

    /** Build the package the test computers run, as the distribution ships it. */
    static runtime(workspace: Workspace): ReturnType<typeof TestDistribution.compile> {
        const directory = fileURLToPath(new URL("../../../", import.meta.url));

        return TestDistribution.compile(workspace, directory);
    }

    /** Build every unit of a workspace package and make its image, once per workspace. */
    static async compile(
        workspace: Workspace,
        directory: string,
    ): Promise<{ readonly image: Digest; readonly units: ReadonlyMap<string, Digest> }> {
        // build and make an image of each unit
        const opened = await workspace.package(directory);
        const imaged = await opened.imageUnits({});
        const units = new Map([...imaged].map(([name, { image }]) => [name, image] as const));

        return { image: found(units, opened.root.name), units };
    }

    /** Select a stack's export at a build's release by its version. */
    static released(reader: BuildReader, exported: string): StackRelease {
        const { id, version } = PackageManifest.package(reader.manifest);

        return {
            packageId: id,
            selection: { kind: "release", version },
            export: exported,
            settings: {},
        };
    }

    /** Close the run's build store. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#store[Symbol.asyncDispose]();
    }
}
