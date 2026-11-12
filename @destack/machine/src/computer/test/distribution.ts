import type { Environment } from "@destack/environment";
import { fileURLToPath } from "node:url";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition, ReleaseRegistry } from "@destack/account/server";
import { CompilerPool, type LocalBuildStore, Workspace } from "@destack/build";
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
import { BuildReader, Lock, PackageManifest } from "@destack/package/manifest";
import { Registry } from "@destack/registry/client";
import { LATEST_TAG } from "@destack/registry/object";
import registryPackage from "@destack/registry/package";
import { Digest, found, present, schema } from "@destack/schema";
import { inject, onTestFinished } from "@destack/test";
import { testHost } from "@destack/test/host";
import { COMPUTER_RUNTIMES } from "../computer.ts";
import type { Machine } from "../../machine/index.ts";
import { MachineError } from "../../error/index.ts";

/** The variable the run's setup writes templates with: its locks' digests by name. */
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
    /** The builds of the stacks, of the packages they install and of the other given builds. */
    readonly builds: readonly BuildReader[];
    /** The manifest of the releases the distribution ships, each with its role. */
    readonly manifest: DistributionManifest;
    /** The platform the manifest has the locks of, the host's. */
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

    /** Ship the run's locks of the platform and its stacks, and the builds a test adds. */
    static async build(
        host: Environment<"files" | "process" | "host">,
        given: readonly BuildReader[] = [],
    ): Promise<TestDistribution> {
        // read each platform package's build at the root of its lock in the run's store
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await testStore(host));
        const built = TestDistribution.built();
        const lockOf = (name: string) => present(built[name], `the run's lock of ${name}`);
        const read = async (name: string) => store.root(lockOf(name));
        const platform = await Promise.all(
            [...PLATFORM_PACKAGES, DESKTOP_PACKAGE].map((name) => read(name)),
        );

        // select the stacks spaces are born with
        const stack = await read(STACK_FIXTURE);
        const start = await read(START_FIXTURE);
        const stacks = {
            home: TestDistribution.released(stack, "HOME_STACK"),
            space: TestDistribution.released(stack, "SPACE_STACK"),
        };
        const residencyHome = TestDistribution.released(stack, "RESIDENCY_STACK");

        // ship the computers' own release, the stacks, the packages they install, and the builds the test adds
        const runtime = await read(RUNTIME_PACKAGE);
        const builds = [runtime, ...platform, stack, start, ...given];
        const extra = await Promise.all(
            given.map((build) => TestDistribution.#lockAt(build, store)),
        );
        const shipped = await TestDistribution.shipped(store, built, extra);
        const target = DistributionPlatform.of(host.host);
        const releases = await Promise.all(
            shipped.map(async (lock) => {
                const root = await store.get((await store.lock(lock)).root);

                return {
                    package: PackageManifest.package(root),
                    role: root.role,
                    locks: { [target]: lock },
                };
            }),
        );
        const layer = await TestDistribution.#distribute(shipped, store);

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

    /** Open a fixture package's build in the run's store for a lifetime, the test's by default. */
    static async fixture(
        fixture: string,
        environment?: Environment<"files" | "process" | "host">,
        defer: (cleanup: () => Promise<void>) => void = onTestFinished,
    ): Promise<BuildReader> {
        // open the store for
        const lock = TestDistribution.lockOf(fixture);
        const store = await testStore(environment);
        defer(() => store[Symbol.asyncDispose]());

        return await store.root(lock);
    }

    /** Read the digests of the run's locks by name: the platform, the stacks and the fixtures. */
    static built(): Readonly<Record<string, Digest>> {
        const written = testHost().process.variables[BUILDS_VARIABLE];
        const built: unknown =
            written === undefined ? (inject("built") ?? {}) : JSON.parse(written);

        return schema.record(schema.string(), Digest).parse(built);
    }

    /** Read the locks of the packages only scenarios install, which no distribution ships. */
    static scenarios(): Readonly<Record<string, Digest>> {
        return schema.record(schema.string(), Digest).parse(inject("scenarios") ?? {});
    }

    /** Find the run's lock rooted at a given build, refusing a build the run did not lock. */
    static async #lockAt(build: BuildReader, store: LocalBuildStore): Promise<Digest> {
        // compare the build with each run lock's root
        const root = await BuildStore.digest(build.manifest, store.environment.crypto);
        const locks = [
            ...Object.values(TestDistribution.built()),
            ...Object.values(TestDistribution.scenarios()),
        ];
        for (const lock of locks) {
            if ((await store.lock(lock)).root === root) {
                return lock;
            }
        }
        throw new TypeError(`the run locked no build '${root}'`);
    }

    /** Read the run's lock of a fixture package. */
    static lockOf(fixture: string): Digest {
        return present(
            TestDistribution.built()[fixture] ?? TestDistribution.scenarios()[fixture],
            `the run's lock of ${fixture}`,
        );
    }

    /** Store a build locked with the builds it runs with, answering the lock's digest. */
    async lock(build: BuildReader, builds: readonly BuildReader[] = []): Promise<Digest> {
        // store each build by its package
        const locked: Record<string, Digest> = {};
        for (const each of builds) {
            locked[each.manifest.package.name] = await this.#store.put(each);
        }
        const root = await this.#store.put(build);

        // lock them hoisted
        const lock = await Lock.hoisted(root, locked, (digest) => this.#store.get(digest));

        return await this.#store.putLock(lock);
    }

    /** Publish a lock in the run's store into its root package's forge as its publisher. */
    async publish(lock: Digest, publisher: TestPublisher, machine: Machine): Promise<Digest> {
        // read the lock's root build
        const { directory, environment } = machine;
        const build = await this.#store.root(lock);

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

        // push its lock and publish it at the test commit
        await registry.release(this.#store, [lock], accountId, LATEST_TAG, TEST_COMMIT);

        return lock;
    }

    /** List the run's locks a distribution ships: the platform, its stacks, builtins and more. */
    static async shipped(
        store: Pick<LocalBuildStore, "get" | "lock">,
        built: Readonly<Record<string, Digest>>,
        given: readonly Digest[],
    ): Promise<Digest[]> {
        // take the named packages' locks and the given ones
        const lockOf = (name: string) => present(built[name], `the run's lock of ${name}`);
        const named = [
            RUNTIME_PACKAGE,
            ...PLATFORM_PACKAGES,
            DESKTOP_PACKAGE,
            STACK_FIXTURE,
            START_FIXTURE,
        ];
        const locks = [...named.map((name) => lockOf(name)), ...given];

        // ship every builtin's lock too
        for (const lock of Object.values(built)) {
            const root = await store.get((await store.lock(lock)).root);
            if (root.role === "builtin") {
                locks.push(lock);
            }
        }

        return [...new Set(locks)];
    }

    /** Write a layer of locked releases once per process as a distribution does, and open it. */
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

    /** Write a linked layer of locked releases in the host's store, returning its directory. */
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
        compilers: CompilerPool,
    ): ReturnType<typeof TestDistribution.compile> {
        const directory = fileURLToPath(new URL(`../../../node_modules/${name}/`, import.meta.url));

        return TestDistribution.compile(compilers, directory);
    }

    /** Build the desktop from its workspace directory, as production's distribution ships it. */
    static desktop(compilers: CompilerPool): ReturnType<typeof TestDistribution.compile> {
        const directory = fileURLToPath(new URL("../../../../desktop/", import.meta.url));

        return TestDistribution.compile(compilers, directory);
    }

    /** Build the package the test computers run, as the distribution ships it. */
    static runtime(compilers: CompilerPool): ReturnType<typeof TestDistribution.compile> {
        const directory = fileURLToPath(new URL("../../../", import.meta.url));

        return TestDistribution.compile(compilers, directory);
    }

    /** Build and lock every unit of a package's workspace. */
    static async compile(
        compilers: CompilerPool,
        directory: string,
    ): Promise<{ readonly lock: Digest; readonly units: ReadonlyMap<string, Digest> }> {
        // build and lock each unit
        const workspace = await Workspace.open(directory, compilers);
        const locked = await workspace.lockUnits({});
        const units = new Map([...locked].map(([name, { lock }]) => [name, lock] as const));

        return { lock: found(units, workspace.root.name), units };
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
