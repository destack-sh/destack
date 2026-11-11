import type { Crypto, Environment } from "@destack/environment";
import { fileURLToPath } from "node:url";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition, ReleaseRegistry } from "@destack/account/server";
import { CompilerPool, type LocalBuildStore, outputsOf, Workspace } from "@destack/build";
import { BuildLayer, BuildStore } from "@destack/build/store";
import { testStore } from "@destack/build/test";
import { FORGE_INSTALLATION } from "@destack/forge/package";
import { InstallationMount, PackageMount } from "@destack/object";
import { createClient } from "@destack/object/client";
import {
    DistributionManifest,
    type PackageId,
    PackageName,
    PackageRelease,
} from "@destack/package";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { Registry } from "@destack/registry/client";
import registryPackage from "@destack/registry/package";
import { Digest, present, schema, type Version } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { inject, onTestFinished } from "@destack/test";
import { testHost } from "@destack/test/host";
import { COMPUTER_RUNTIMES } from "../computer.ts";
import type { Machine } from "../../machine/index.ts";
import { MachineError } from "../../error/index.ts";

/** The variable the run's setup writes templates with: its builds' digests by name. */
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
    /** The builds with what they run, as a machine ships them in place. */
    readonly layer: BuildLayer;
    /** The run's build store the server builds read from, open until the distribution closes. */
    readonly #store: LocalBuildStore;

    /** The layers this process writes or wrote, by their releases, each written once. */
    static readonly #distributed = new Map<string, Promise<string>>();

    /** Take the stacks, the builds, the shipped releases, their layer and their store. */
    private constructor(
        stacks: PlatformDefinition["stacks"],
        home: StackRelease,
        builds: readonly BuildReader[],
        manifest: DistributionManifest,
        layer: BuildLayer,
        store: LocalBuildStore,
    ) {
        // set the stacks, the residency homes', the builds, the shipped releases, their layer and their store
        this.stacks = stacks;
        this.home = home;
        this.builds = builds;
        this.manifest = manifest;
        this.layer = layer;
        this.#store = store;
    }

    /** Ship the run's builds of the platform's packages and stacks, and the builds a test adds. */
    static async build(
        host: Environment<"files" | "process" | "host">,
        given: readonly BuildReader[] = [],
    ): Promise<TestDistribution> {
        // read each platform package's build from the run's store
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await testStore(host));
        const { crypto } = host;
        const built = TestDistribution.built();
        const read = async (name: string) =>
            store.contents(present(built[name], `the run's build of ${name}`));
        const platform = await Promise.all(
            [...PLATFORM_PACKAGES, DESKTOP_PACKAGE].map((name) => read(name)),
        );

        // pin the stacks spaces are born with, and the residencies' homes', from the stack fixture
        const stack = await read(STACK_FIXTURE);
        const start = await read(START_FIXTURE);
        const stacks = {
            home: await TestDistribution.pinned(stack, "HOME_STACK", crypto),
            space: await TestDistribution.pinned(stack, "SPACE_STACK", crypto),
        };
        const residencyHome = await TestDistribution.pinned(stack, "RESIDENCY_STACK", crypto);

        // ship the computers' own release, the stacks, the packages they install, and the builds the test adds
        const runtime = await read(RUNTIME_PACKAGE);
        const builds = [runtime, ...platform, stack, start, ...given];

        // store the shipped builds in the run's store
        const manifests = [];
        for (const build of builds) {
            manifests.push(await store.put(build));
        }

        // ship the Destack releases they and the run's other builds were built against, as a distribution ships every platform release
        const others = await Promise.all(
            Object.values(built).map((digest) => store.contents(digest)),
        );
        const recorded = new Set(
            (await Promise.all([...builds, ...others].map((build) => build.releases())))
                .flat()
                .map((each) => each.manifest),
        );
        manifests.push(...recorded);

        // list the shipped releases with their roles, and write their builds with what they run as a distribution's layer
        const releases = await Promise.all(
            [...new Set(manifests)].map(async (manifest) => {
                const reader = await store.contents(manifest);

                return {
                    package: PackageManifest.package(reader.manifest),
                    manifest,
                    role: reader.manifest.role,
                };
            }),
        );
        const layer = await TestDistribution.#distribute(manifests, store);

        // hand the store to the distribution
        opened.move();

        return new TestDistribution(stacks, residencyHome, builds, { releases }, layer, store);
    }

    /** Open a fixture package's build in the run's store for a lifetime, the test's by default. */
    static async fixture(
        fixture: string,
        environment?: Environment<"files" | "process" | "host">,
        defer: (cleanup: () => Promise<void>) => void = onTestFinished,
    ): Promise<BuildReader> {
        // open the store for as long as the lifetime reads the build
        const built = present(
            TestDistribution.built()[fixture] ?? TestDistribution.scenarios()[fixture],
            `the run's build of ${fixture}`,
        );
        const store = await testStore(environment);
        defer(() => store[Symbol.asyncDispose]());

        return await store.contents(Digest.parse(built));
    }

    /** Read the digests of the run's builds by name: the platform, the stacks and the fixtures. */
    static built(): Readonly<Record<string, Digest>> {
        const written = testHost().process.variables[BUILDS_VARIABLE];
        const built: unknown =
            written === undefined ? (inject("built") ?? {}) : JSON.parse(written);

        return schema.record(schema.string(), Digest).parse(built);
    }

    /** Read the digests of the packages only scenarios install, which no distribution ships. */
    static scenarios(): Readonly<Record<string, Digest>> {
        return schema.record(schema.string(), Digest).parse(inject("scenarios") ?? {});
    }

    /** Publish a build into its scope's forge as its publisher, as `destack publish` does. */
    async publish(build: BuildReader, publisher: TestPublisher, machine: Machine): Promise<Digest> {
        // store the build in the run's store, which the push reads it from
        const { directory, environment } = machine;
        const manifest = await this.#store.put(build);

        // wait until the machine's replica of the directory has the account of the package's scope
        const { id, name } = PackageManifest.package(build.manifest);
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
        const forge = createClient(registryPackage, { url, fetch }, environment);
        const registry = new Registry(async () => ({ url, space: home, fetch }), environment);

        // create the package once, push its build and release it
        await forge.package.create({
            spaceId: home,
            requestId: RequestId.create(environment),
            id,
            accountId,
            name: parsed.local,
        });
        await registry.push(this.#store, manifest);
        await forge.release.create({
            spaceId: home,
            requestId: RequestId.create(environment),
            parentId: id,
            manifest,
        });

        return manifest;
    }

    /** Write a layer of releases once per process as a distribution does, and open it. */
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

    /** Write a linked layer of releases in the host's store, returning its directory. */
    static async #write(releases: readonly Digest[]): Promise<string> {
        await using store = await testStore();

        return await store.writeLinkedLayer(releases, COMPUTER_RUNTIMES);
    }

    /** The releases the distribution ships, which a scope runs of a package it installs none of. */
    get distribution(): readonly PackageRelease[] {
        return DistributionManifest.releases(this.manifest);
    }

    /** Open a shipped release of a package by its version, refusing one the distribution lacks. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        const found = this.builds.find(({ manifest }) => {
            const shipped = manifest.package;

            return "id" in shipped && shipped.id === packageId && shipped.version === release;
        });
        if (found === undefined) {
            throw MachineError.of(
                "PACKAGE_NOT_FOUND",
                `release '${release}' of this package on the test computer not found`,
            );
        }

        return found;
    }

    /** Build a platform package from the workspace, found through machine's dependency on it. */
    static platform(
        name: (typeof PLATFORM_PACKAGES)[number],
        compilers: CompilerPool,
    ): Promise<BuildReader> {
        const directory = fileURLToPath(new URL(`../../../node_modules/${name}/`, import.meta.url));

        return TestDistribution.compile(compilers, directory);
    }

    /** Build the desktop from its workspace directory, as production's distribution ships it. */
    static desktop(compilers: CompilerPool): Promise<BuildReader> {
        const directory = fileURLToPath(new URL("../../../../desktop/", import.meta.url));

        return TestDistribution.compile(compilers, directory);
    }

    /** Build the package the test computers run, as the distribution ships it. */
    static runtime(compilers: CompilerPool): Promise<BuildReader> {
        const directory = fileURLToPath(new URL("../../../", import.meta.url));

        return TestDistribution.compile(compilers, directory);
    }

    /** Build every output a package declares into the run's store at some versions, and read it. */
    static async compile(
        compilers: CompilerPool,
        directory: string,
        versions: Readonly<Record<string, Version>> = {},
    ): Promise<BuildReader> {
        // build every output the package's exports declare, as its release and its dependents' builds do
        const workspace = await Workspace.open(directory, compilers);

        return await workspace.build({ outputs: outputsOf(workspace.root.packageJson), versions });
    }

    /** Pin a stack's export to a build, as a stack release by its version and manifest. */
    static async pinned(
        reader: BuildReader,
        exported: string,
        crypto: Crypto,
    ): Promise<StackRelease> {
        const { id, version } = PackageManifest.package(reader.manifest);
        const manifest = await BuildStore.digest(reader.manifest, crypto);

        return {
            packageId: id,
            selection: { kind: "release", version, manifest },
            export: exported,
            settings: {},
        };
    }

    /** Close the run's build store. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#store[Symbol.asyncDispose]();
    }
}
