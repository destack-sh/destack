import { type Crypto, type Environment, FileSystemError } from "@destack/environment";
import { install } from "@destack/stack/declare";
import { join } from "node:path";
import homePackage from "@app/home/package";
import type { StackRelease } from "@destack/account/object";
import type { PlatformDefinition } from "@destack/account/server";
import connectionPackage, { CONNECTION_INSTALLATION } from "@destack/connection/package";
import { CompilerPool, type LocalBuildStore, outputsOf, Workspace } from "@destack/build";
import { BuildLayer, BuildStore } from "@destack/build/store";
import { testStore } from "@destack/build/test";
import { COMPUTER_RUNTIMES } from "../computer.ts";
import { TestRegistry } from "@destack/stack/test";
import forgePackage from "@destack/forge/package";
import { credentialVault, github, repositoryStorage } from "@destack/forge/stack";
import { gitStorage } from "@destack/forge/storage";
import messagePackage from "@destack/message/package";
import notificationPackage from "@destack/notification/package";
import {
    OBJECT_DECLARATION,
    ObjectDescription,
    type ObjectType,
    PACKAGE_DECLARATION,
    PACKAGE_EXPORT,
    type PackageHandle,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import observabilityPackage from "@destack/observability/package";
import {
    type DependencyRelease,
    graph,
    Package,
    type PackageId,
    type PackageRelease,
} from "@destack/package";
import {
    BuildReader,
    PackageManifest,
    type ViewDescription,
    type ViewRender,
} from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import {
    RESOURCE_DECLARATION,
    RESOURCE_KIND_PACKAGE_ID,
    ResourceDeclaration,
} from "@destack/resource";
import { Commit, Digest, type JsonObject, schema } from "@destack/schema";
import { inject } from "@destack/test";
import { testHost } from "@destack/test/host";
import { describeSetting } from "@destack/setting/inspect";
import { setting } from "@destack/setting/object";
import socialPackage from "@destack/social/package";
import { defineStack, type StackDeclaration, type StackDocument } from "@destack/stack";
import stackPackage from "@destack/stack/package";
import type { ReleaseRegistry } from "@destack/account/server";
import { defineVault, SecretDeclaration } from "@destack/vault/declare";
import vaultPackage from "@destack/vault/package";
import { TestVault } from "@destack/vault/test";
import { View } from "@destack/view/declare";
import { MachineError } from "../../error/index.ts";

/** The variable the run's setup writes templates with: the fixtures' digests by name. */
export const FIXTURES_VARIABLE = "DESTACK_MACHINE_FIXTURES";

/** The public origin a shipped static view declares for its pages. */
const SITE_ORIGIN = "https://example.test";

/** Home's views with their roles, as the stand-in of Home's release declares them. */
const HOME_VIEWS: Readonly<Record<string, ViewDescription>> = Object.fromEntries(
    (
        [
            ["home", ["home"]],
            ["settings", []],
            ["command", ["command"]],
        ] as const
    ).map(([name, categories]) => [
        name,
        {
            entrypoint: `browser/${name}.js`,
            stylesheets: [],
            sandbox: {},
            presents: [],
            categories: [...categories],
            packages: [],
            render: { mode: "client" as const },
        },
    ]),
);

/** The main module a view build's browser output exports, as a compiled view's index. */
const BROWSER_MAIN = "browser/index.js";

/** The stand-in stack package of every personal home. */
const HOME_STACK = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c7",
    name: "@destack/machine-home-fixture",
    version: "2026.9.0",
});

/** The stand-in stack package of every other space. */
const SPACE_STACK = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c9",
    name: "@destack/machine-space-fixture",
    version: "2026.9.0",
});

/** The stand-in stack package of every other account's home. */
const ACCOUNT_STACK = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000ca",
    name: "@destack/machine-account-fixture",
    version: "2026.9.0",
});

/** The stand-in stack package of each residency's home. */
const RESIDENCY_STACK = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000c8",
    name: "@destack/machine-residency-fixture",
    version: "2026.9.0",
});

/** The commit the in-memory builds record, as a release of a committed source does. */
const SHIPPED_COMMIT = Commit.parse("a".repeat(40));

/** The vault of the residencies' homes, whose secrets the forge binds. */
const PLATFORM_VAULT = defineVault(
    { name: "platform", spec: {} },
    { package: RESIDENCY_STACK },
);

/** The vault of every account's home that stores its connections' credentials. */
const CONNECTION_VAULT = defineVault({ name: "credentials", spec: {} }, { package: ACCOUNT_STACK });

/** What every account's home runs, as the platform's: its connections and their vault. */
const ACCOUNT_HOME = {
    vaults: { credentials: { ...CONNECTION_VAULT.declare(), scope: "enclosing" } },
    installations: {
        vault: install(vaultPackage, {}, { name: "vault" }),
        connection: install(connectionPackage, {}, { name: CONNECTION_INSTALLATION }),
    },
} satisfies StackDocument<readonly [typeof vaultPackage]>;

/** The stack of every other account's home, as the platform's: its connections. */
const ACCOUNT_SPACE = defineStack([vaultPackage], ACCOUNT_HOME, { package: ACCOUNT_STACK });

/** The stack of every personal home, as the platform's: an account's home with Home. */
const PERSONAL_SPACE = defineStack(
    [vaultPackage],
    {
        ...ACCOUNT_HOME,
        installations: {
            ...ACCOUNT_HOME.installations,
            home: install(homePackage, {}, { name: "home" }),
            inbox: install(notificationPackage, {}, { name: "inbox", capabilities: ["deliver"] }),
            observability: install(observabilityPackage, {}, { name: "observability" }),
        },
    },
    { package: HOME_STACK },
);

/** The stack of every other space, as the platform's space stack: observability. */
const OTHER_SPACE = defineStack(
    [],
    {
        installations: {
            observability: install(observabilityPackage, {}, { name: "observability" }),
        },
    },
    { package: SPACE_STACK },
);

/** The stack of a residency's home that runs the forge. */
const FORGE_SPACE = defineStack(
    [vaultPackage, forgePackage, gitStorage],
    {
        vaults: { platform: PLATFORM_VAULT.declare(), credentials: credentialVault.declare() },
        gitStorages: { repositories: repositoryStorage.declare() },
        issuers: { github: {} },
        secrets: {
            "github-webhook": { parent: "platform", name: "github-webhook" },
            "github-private-key": { parent: "platform", name: "github-private-key" },
        },
        installations: {
            vault: install(vaultPackage, {}, { name: "vault" }),
            forge: install(
                forgePackage,
                {
                    repositories: "repositories",
                    credentials: "credentials",
                    "github-webhook": "github-webhook",
                    "github-private-key": "github-private-key",
                },
                { name: "forge" },
            ),
        },
    },
    { package: RESIDENCY_STACK },
);

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

    /** The layers this process writes or wrote, by directory, each written once. */
    static readonly #distributed = new Map<string, Promise<void>>();

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

    /** Report whether a given build replaces a package the distribution's stacks pin. */
    static isReplacing(given: readonly BuildReader[]): boolean {
        const shipped = new Set(
            [
                homePackage,
                messagePackage,
                notificationPackage,
                connectionPackage,
                socialPackage,
                observabilityPackage,
                vaultPackage,
                forgePackage,
            ].map((handle) => handle.id),
        );

        return given.some((reader) => shipped.has(PackageManifest.package(reader.manifest).id));
    }

    /** Build the platform's stacks against their releases, given builds replacing stand-ins. */
    static async build(
        host: Environment<"files" | "process" | "host">,
        given: readonly BuildReader[] = [],
    ): Promise<TestDistribution> {
        // ship each package's given build, else its stand-in, the store closed on a failure
        await using opened = new AsyncDisposableStack();
        const store = opened.use(await testStore(host));
        const { crypto } = host;
        const release = async (
            handle: { readonly id: string },
            standIn: () => Promise<BuildReader>,
        ) =>
            given.find((reader) => PackageManifest.package(reader.manifest).id === handle.id) ??
            (await standIn());

        // ship Home with its views, and the vault, messages, the inbox and connections their servers run in process for
        const home = await release(homePackage, () =>
            TestDistribution.view(homePackage, HOME_VIEWS, { package: homePackage }, crypto),
        );
        const vault = await release(vaultPackage, () =>
            TestDistribution.view(vaultPackage, {}, { package: vaultPackage }, crypto),
        );
        const message = await release(messagePackage, () =>
            TestDistribution.view(messagePackage, {}, { package: messagePackage }, crypto),
        );
        const inbox = await release(notificationPackage, () =>
            TestDistribution.view(
                notificationPackage,
                {},
                { package: notificationPackage, dependencies: [message, vault] },
                crypto,
            ),
        );
        const connection = await release(connectionPackage, () =>
            TestDistribution.view(
                connectionPackage,
                {},
                { package: connectionPackage, dependencies: [vault] },
                crypto,
            ),
        );

        // ship social and observability against them
        const social = await release(socialPackage, () =>
            TestDistribution.view(
                socialPackage,
                {},
                {
                    package: socialPackage,
                    dependencies: [inbox, message],
                },
                crypto,
            ),
        );
        const observability = await release(observabilityPackage, () =>
            TestDistribution.view(
                observabilityPackage,
                {},
                {
                    package: observabilityPackage,
                    dependencies: [inbox, social, message],
                },
                crypto,
            ),
        );

        // ship the forge, declaring its GitHub App setting
        const forge = await release(forgePackage, () =>
            TestDistribution.view(
                forgePackage,
                {},
                {
                    package: forgePackage,
                    declarations: [
                        {
                            kind: "setting",
                            kindPackageId: setting.package.id,
                            name: github.name,
                            description: describeSetting(github),
                        },
                    ],
                },
                crypto,
            ),
        );

        // declare the personal and other accounts' homes, every other space and the residencies' homes
        const account = await TestDistribution.stack(
            ACCOUNT_STACK,
            "ACCOUNT",
            ACCOUNT_SPACE,
            [
                await TestDistribution.release(vault, crypto),
                await TestDistribution.release(connection, crypto),
            ],
            crypto,
        );
        const personal = await TestDistribution.stack(
            HOME_STACK,
            "HOME",
            PERSONAL_SPACE,
            [
                await TestDistribution.release(vault, crypto),
                await TestDistribution.release(connection, crypto),
                await TestDistribution.release(home, crypto),
                await TestDistribution.release(inbox, crypto),
                await TestDistribution.release(observability, crypto),
            ],
            crypto,
        );
        const other = await TestDistribution.stack(
            SPACE_STACK,
            "SPACE",
            OTHER_SPACE,
            [await TestDistribution.release(observability, crypto)],
            crypto,
        );
        const residency = await TestDistribution.stack(
            RESIDENCY_STACK,
            "RESIDENCY",
            FORGE_SPACE,
            [
                await TestDistribution.release(vault, crypto),
                await TestDistribution.release(forge, crypto),
            ],
            crypto,
        );

        // pin the stacks the spaces are born with
        const installed = [home, message, inbox, connection, social, observability, vault, forge];
        const stacks = {
            person: await TestDistribution.pinned(personal, "HOME", crypto),
            account: await TestDistribution.pinned(account, "ACCOUNT", crypto),
            space: await TestDistribution.pinned(other, "SPACE", crypto),
        };

        // release the residency home's stack
        const residencyHome = await TestDistribution.pinned(residency, "RESIDENCY", crypto);

        // ship the stacks, the packages they install, and the other given builds
        const builds = [
            ...installed,
            account,
            personal,
            residency,
            other,
            ...given.filter((reader) => !installed.includes(reader)),
        ];

        // serve the shipped releases from the run's store of their closures' builds
        const registry = new TestRegistry(store);
        const manifests = [];
        for (const build of builds) {
            manifests.push(await registry.release(build));
        }

        // publish the Destack releases they and the run's fixtures were built against, and ship them, as a distribution ships every platform release
        const fixtures = await Promise.all(
            Object.values(TestDistribution.fixtures()).map((digest) => store.contents(digest)),
        );
        const recorded = new Set(
            (await Promise.all([...builds, ...fixtures].map((build) => build.releases())))
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

    /** Read the digests of the fixtures the run's setup built, by name. */
    static fixtures(): Readonly<Record<string, Digest>> {
        const written = testHost().process.variables[FIXTURES_VARIABLE];
        const fixtures: unknown =
            written === undefined ? (inject("fixtures") ?? {}) : JSON.parse(written);

        return schema.record(schema.string(), Digest).parse(fixtures);
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

    /** Write a layer of releases once per store as a distribution does, and open it. */
    static async #distribute(
        releases: readonly Digest[],
        store: LocalBuildStore,
    ): Promise<BuildLayer> {
        // write it once per process, beside the store under its releases' digest
        const { files, crypto, process } = store.environment;
        const key = await Digest.json({ releases, runtimes: COMPUTER_RUNTIMES }, crypto);
        const directory = join(store.directory, "layers", key);
        const writing =
            TestDistribution.#distributed.get(directory) ??
            (async () => {
                // write it under a name of this process's own, then move it in, leaving one another process moved in first
                if (await files.exists(directory)) {
                    return;
                }
                const written = `${directory}.${process.id}`;
                await files.remove(written, { recursive: true, force: true });
                await store.writeLayer(releases, COMPUTER_RUNTIMES, written);
                await files.rename(written, directory).catch(async (error: unknown) => {
                    if (!FileSystemError.is(error, "ALREADY_EXISTS")) {
                        throw error;
                    }
                    await files.remove(written, { recursive: true });
                });
            })();
        TestDistribution.#distributed.set(directory, writing);
        await writing;

        return await BuildLayer.open(directory, store.environment);
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

    /** Write a package's build in memory with a browser output that mounts views. */
    static async view(
        built: Package,
        views: Readonly<Record<string, ViewDescription>>,
        options: {
            /** The build's other files, by path. */
            readonly files?: ReadonlyMap<string, Uint8Array<ArrayBuffer>>;
            /** The package's definition, declared as its build would, with its object types. */
            readonly package?: PackageHandle;
            /** The object types the package declares, its definition's when absent. */
            readonly objects?: readonly ObjectType[];
            /** The builds of the Destack packages it depends on, pinned with their closures. */
            readonly dependencies?: readonly BuildReader[];
            /** The other declarations it makes, such as commands, each at an export. */
            readonly declarations?: readonly Omit<graph.Declaration, "moniker" | "symbol">[];
        },
        crypto: Crypto,
    ): Promise<BuildReader> {
        // write its main module and each view's beside the other files
        const module = new TextEncoder().encode("export {};\n");
        const files = new Map(options.files);
        files.set(BROWSER_MAIN, module);
        for (const view of Object.values(views)) {
            files.set(view.entrypoint, module);
        }

        // declare the package, its object types, its resources and its views, as its build would
        const objects = options.objects ?? Object.values(options.package?.objects ?? {});
        const declared = [
            ...(options.package === undefined
                ? []
                : [
                      {
                          kind: PACKAGE_DECLARATION,
                          kindPackageId: objectPackage.id,
                          name: PACKAGE_EXPORT,
                          description: describePackage(options.package),
                      },
                  ]),
            ...objects.map((type) => ({
                kind: OBJECT_DECLARATION,
                kindPackageId: objectPackage.id,
                name: type.name,
                description: ObjectDescription.of(type),
            })),
            ...Object.values(options.package?.resources ?? {})
                .filter((value) => ResourceDeclaration.is(value))
                .map((resource) => ({
                    kind: RESOURCE_DECLARATION,
                    kindPackageId: RESOURCE_KIND_PACKAGE_ID,
                    name: resource.name,
                    description: {
                        name: resource.name,
                        kind: resource.kind,
                        spec: resource.spec,
                        ...resource.state(),
                    },
                })),
            ...TestVault.declarations(
                [],
                Object.values(options.package?.secrets ?? {}).filter(
                    (value) => value instanceof SecretDeclaration,
                ),
            ),
            ...Object.entries(views).map(([name, view]) => ({
                ...View.kind,
                name,
                description: {
                    name,
                    presents: view.presents,
                    categories: view.categories,
                    packages: view.packages,
                    render: TestDistribution.#declared(view.render),
                },
            })),
            ...(options.declarations ?? []),
        ];

        // pin the Destack packages it depends on with the closures they resolved, as a build records them
        const given = options.dependencies ?? [];
        const direct = await Promise.all(
            given.map((each) => TestDistribution.release(each, crypto)),
        );
        const resolved = (await Promise.all(given.map((each) => each.releases()))).flat();
        const dependencies = TestDistribution.#pin([...direct, ...resolved]);

        // export the main module and each view's, each importing the direct dependencies
        const imports = direct.map((release) => release.package.name);
        const exports: Record<string, { path: string; imports: string[] }> = {
            ".": { path: BROWSER_MAIN, imports },
        };
        for (const [name, view] of Object.entries(views)) {
            exports[`./${name}`] = { path: view.entrypoint, imports };
        }

        return MemoryBuild.of(built, declared, crypto, {
            files,
            commit: SHIPPED_COMMIT,
            dependencies,
            outputs: {
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views,
                    directory: "browser",
                    exports,
                    // pin the direct ones in its browser output, as a view's code imports them
                    dependencies: TestDistribution.#pin(direct),
                    peers: {},
                },
            },
        });
    }

    /** Describe how a shipped view's declaration renders, a static one at the test origin. */
    static #declared(render: ViewRender): JsonObject {
        // describe a static view by the test origin, hydrating its pages whole
        if (render.mode === "static") {
            return { mode: "static", origin: SITE_ORIGIN, hydrate: "page" };
        }

        // describe a client or server view by its mode
        return { mode: render.mode };
    }

    /** Pin releases as a build's Destack dependencies, by package name. */
    static #pin(releases: readonly PackageRelease[]): Record<string, DependencyRelease> {
        return Object.fromEntries(
            releases.map((release) => [release.package.name, { ...release, kind: "destack" }]),
        );
    }

    /** Write a stack package's build in memory that declares one stack at an export. */
    static async stack(
        built: Package,
        name: string,
        declared: StackDeclaration,
        releases: readonly PackageRelease[],
        crypto: Crypto,
    ): Promise<BuildReader> {
        // declare the stack at an export, pinning the releases it installs
        const dependencies = Object.fromEntries(
            releases.map((release) => [
                release.package.name,
                { ...release, kind: "destack" as const },
            ]),
        );

        return MemoryBuild.of(
            built,
            [
                {
                    kind: "stack",
                    kindPackageId: stackPackage.id,
                    name,
                    description: declared.definition,
                },
            ],
            crypto,
            { commit: SHIPPED_COMMIT, dependencies },
        );
    }

    /** Build a package's declared outputs into the run's store, and read the stored build. */
    static async server(store: LocalBuildStore, directory: string): Promise<BuildReader> {
        // build every output the package's exports declare, as its release and its dependents' builds do
        await using compilers = await CompilerPool.open(store);
        const workspace = await Workspace.open(directory, compilers);

        return await workspace.build({ outputs: outputsOf(workspace.root.packageJson) });
    }

    /** Name a build as the release a stack pins: its package and its manifest's digest. */
    static async release(reader: BuildReader, crypto: Crypto): Promise<PackageRelease> {
        return {
            package: PackageManifest.package(reader.manifest),
            manifest: await BuildStore.digest(reader.manifest, crypto),
        };
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
