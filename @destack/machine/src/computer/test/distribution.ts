import type { Crypto, Environment } from "@destack/environment";
import { fileURLToPath } from "node:url";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition, ReleaseRegistry } from "@destack/account/server";
import { CompilerPool, type LocalBuildStore, outputsOf, Workspace } from "@destack/build";
import { BuildLayer, BuildStore } from "@destack/build/store";
import { testStore } from "@destack/build/test";
import type { PackageId } from "@destack/package";
import { BuildReader, PackageManifest } from "@destack/package/manifest";
import { Digest, present, schema } from "@destack/schema";
import { TestRegistry } from "@destack/stack/test";
import { inject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { COMPUTER_RUNTIMES } from "../computer.ts";
import { MachineError } from "../../error/index.ts";

/** The variable the run's setup writes templates with: its builds' digests by name. */
export const BUILDS_VARIABLE = "DESTACK_MACHINE_BUILDS";

/** The platform packages a test distribution ships, built from the workspace as releases are. */
export const PLATFORM_PACKAGES = [
    "@app/home",
    "@destack/vault",
    "@destack/message",
    "@destack/notification",
    "@destack/connection",
    "@destack/social",
    "@destack/observability",
    "@destack/forge",
] as const;

/** The fixture package declaring the stacks spaces are born with, as the platform's does. */
export const STACK_FIXTURE = "stack";

/** What a test computer ships: the platform's stacks, what they install, and other builds. */
export class TestDistribution implements ReleaseRegistry, AsyncDisposable {
    /** The stacks spaces are born with by kind. */
    readonly stacks: PlatformDefinition["stacks"];
    /** The stack each residency's home runs. */
    readonly home: StackRelease;
    /** The builds of the stacks, of the packages they install and of the other given builds. */
    readonly builds: readonly BuildReader[];
    /** The builds with what they run, as a machine ships them in place. */
    readonly layer: BuildLayer;
    /** The registry serving the shipped releases from the run's build store. */
    readonly registry: TestRegistry;
    /** The run's build store the server builds read from, open until the distribution closes. */
    readonly #store: LocalBuildStore;

    /** The layers this process writes or wrote, by their releases, each written once. */
    static readonly #distributed = new Map<string, Promise<string>>();

    /** Take the stacks, the builds, their registry and the store they read from. */
    private constructor(
        stacks: PlatformDefinition["stacks"],
        home: StackRelease,
        builds: readonly BuildReader[],
        layer: BuildLayer,
        registry: TestRegistry,
        store: LocalBuildStore,
    ) {
        // set the stacks, the residency homes', the builds, their layer, the registry and its store
        this.stacks = stacks;
        this.home = home;
        this.builds = builds;
        this.layer = layer;
        this.registry = registry;
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
        const platform = await Promise.all(PLATFORM_PACKAGES.map((name) => read(name)));

        // pin the stacks spaces are born with, and the residencies' homes', from the stack fixture
        const stack = await read(STACK_FIXTURE);
        const stacks = {
            person: await TestDistribution.pinned(stack, "PERSON_STACK", crypto),
            account: await TestDistribution.pinned(stack, "ACCOUNT_STACK", crypto),
            space: await TestDistribution.pinned(stack, "SPACE_STACK", crypto),
        };
        const residencyHome = await TestDistribution.pinned(stack, "RESIDENCY_STACK", crypto);

        // ship the stacks, the packages they install, and the builds the test adds
        const builds = [...platform, stack, ...given];

        // serve the shipped releases from the run's store of their closures' builds
        const registry = new TestRegistry(store);
        const manifests = [];
        for (const build of builds) {
            manifests.push(await registry.release(build));
        }

        // publish the Destack releases they and the run's other builds were built against, and ship them, as a distribution ships every platform release
        const others = await Promise.all(
            Object.values(built).map((digest) => store.contents(digest)),
        );
        const recorded = new Set(
            (await Promise.all([...builds, ...others].map((build) => build.releases())))
                .flat()
                .map((each) => each.manifest),
        );
        for (const manifest of recorded) {
            manifests.push(await registry.release(await store.contents(manifest)));
        }

        // ship the releases with their runtime builds and what those run, written and opened as a distribution's
        const layer = await TestDistribution.#distribute(manifests, store);

        // hand the store to the distribution
        opened.move();

        return new TestDistribution(stacks, residencyHome, builds, layer, registry, store);
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

    /** Publish a build with the Destack releases its closure was built against. */
    async publish(build: BuildReader): Promise<Digest> {
        // push each release the closure records from the run's store, each once
        const pushed = new Set<Digest>();
        const pending = [...(await build.releases())];
        for (let next = pending.pop(); next !== undefined; next = pending.pop()) {
            if (!pushed.has(next.manifest)) {
                pushed.add(next.manifest);
                await this.registry.push(this.#store, next.manifest);
                pending.push(...(await (await this.#store.contents(next.manifest)).releases()));
            }
        }

        return await this.registry.release(build);
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

    /** Open a shipped release of a package by its version, refusing one the distribution lacks. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        const found = this.builds.find(({ manifest }) => {
            const shipped = manifest.package;

            return "id" in shipped && shipped.id === packageId && shipped.version === release;
        });
        if (found === undefined) {
            throw MachineError.of(
                "PACKAGE_NOT_SERVED",
                `the test computer ships no release ${release} of ${packageId}`,
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

    /** Build every output a package declares into the run's store, and read the stored build. */
    static async compile(compilers: CompilerPool, directory: string): Promise<BuildReader> {
        // build every output the package's exports declare, as its release and its dependents' builds do
        const workspace = await Workspace.open(directory, compilers);

        return await workspace.build({ outputs: outputsOf(workspace.root.packageJson) });
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
