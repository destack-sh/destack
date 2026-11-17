import type { Runtime } from "@destack/package/runtime";
import { Failure } from "@destack/error";
import { BucketKind } from "@destack/bucket/declare";
import { InstallationKind, type InstallationSelection } from "@destack/object";
import type { Crypto } from "@destack/environment";
import { StackDefinition } from "@destack/stack/declare";
import { StackError } from "@destack/stack/error";
import { type Digest, type JsonObject, present, schema } from "@destack/schema";
import { ObjectReference, Scope } from "@destack/sync";
import { release, tag } from "@destack/registry/object";
import { PackageVersion, PackageId, type PackageRelease } from "@destack/package";
import {
    type BuildReader,
    Lock,
    OutputContent,
    type OutputExport,
} from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import type { Registry } from "../index.ts";
import type { ReleaseRegistry } from "@destack/account/server";
import { BuildStore } from "@destack/build/store";
import { PackageManifest } from "@destack/package/manifest";

/** The releases the fixture's distribution tags point at, as the forge resolves `<name>@<tag>`. */
export const TAGS: Readonly<Record<string, string>> = { latest: "2026.10.0" };

/** The workload every fixture build's server output has. */
export const workload = {
    entrypoint: ".",
    packages: [],
    resources: [],
    secrets: [],
    compute: {},
    sandbox: {},
};

/** What each release of the fixture declares beside its archive command, by release. */
const DECLARED: Readonly<
    Record<
        string,
        (packageId: PackageId) => readonly {
            readonly kind: string;
            readonly kindPackageId?: string;
            readonly provisioned?: true;
            readonly name: string;
            readonly description?: Readonly<Record<string, unknown>>;
        }[]
    >
> = {
    "2026.9.0": () => [{ kind: "relation", name: "editor" }],
    "2026.9.1": () => [
        { kind: "relation", name: "editor" },
        ...["ledger", "archive"].map((name) => ({
            ...BucketKind.identity,
            provisioned: true as const,
            name,
            description: { name, spec: {} },
        })),
    ],
    "2026.9.2": () => [
        { kind: "relation", name: "editor" },
        {
            ...BucketKind.identity,
            provisioned: true as const,
            name: "files",
            description: { name: "files", spec: {} },
        },
    ],
    "2026.9.3": (packageId) => [
        { kind: "relation", name: "editor" },
        {
            ...InstallationKind.identity,
            provisioned: true as const,
            name: "tasks",
            description: { name: "tasks", spec: { package: packageId } },
        },
    ],
    "2026.10.0": () => [{ kind: "relation", name: "editor" }],
    "2026.11.0": () => [{ kind: "procedure", name: "count" }],
    "2026.12.0": () => [{ kind: "procedure", name: "count" }],
};

/** Complete fixture build outputs with their directory, exporting each workload's entrypoint. */
export function fixtureOutputs(
    outputs: Readonly<Record<string, Readonly<Record<string, unknown>>>>,
): Readonly<Record<string, OutputContent>> {
    return Object.fromEntries(
        Object.entries(outputs).map(([name, output]) => {
            // export each workload's entrypoint as a module importing no dependency, as a build does
            const parsed = OutputContent.parse({
                directory: name,
                exports: {},
                peers: {},
                ...output,
            });
            const exported = Object.values(parsed.workloads).map(
                (declared): [string, OutputExport] => [
                    declared.entrypoint,
                    { path: `${name}/index.js`, imports: [] },
                ],
            );

            return [
                name,
                OutputContent.parse({
                    ...parsed,
                    exports: { ...Object.fromEntries(exported), ...parsed.exports },
                }),
            ];
        }),
    );
}

/** The home space of the fixture registry's forge, which no machine of a test world serves. */
const FORGE_HOME = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f0");

/** The hex digits a UUIDv8 leaves free past its version and variant: a fixture name's 9 bytes. */
const FIXTURE_NAME_DIGITS = 18;

/** The registry the fixture's releases come from. */
export class TestRegistry implements Registry, ReleaseRegistry {
    /** The stored fixture builds. */
    readonly #store: BuildStore;
    /** The locks of the pushed releases, by package and version. */
    readonly #pushed = new Map<string, Digest>();

    /** The fixture app whose declared releases it serves, absent where it serves released ones. */
    readonly #fixture: PackageId | undefined;
    /** The forges a served machine calls for repositories, absent before one serves. */
    #forges: Pick<Registry, "followed"> | undefined;

    /** Serve the releases released to it from a store, and a fixture app's declared releases. */
    constructor(store: BuildStore, fixture?: PackageId) {
        this.#store = store;
        this.#fixture = fixture;
    }

    /** Refuse npm clients, which the fixture registry serves no packages to. */
    async npm(handle: string): Promise<string> {
        throw new Error(`fixture registry serves no npm packages of '@${handle}'`);
    }

    /** Find a release at a version or a tag: one released to it, or the fixture app's. */
    async find(packageId: PackageId, named: string): Promise<Digest> {
        // find a release published to the registry
        const version = TAGS[named] ?? named;
        const pushed = this.#pushed.get(`${packageId}@${version}`);
        if (pushed !== undefined) {
            return pushed;
        }
        // serve a declared release of the fixture app, built as its forge stored it
        else if (packageId === this.#fixture && Object.hasOwn(DECLARED, version)) {
            const crypto = this.#store.environment.crypto;
            const root = await this.#store.put(await fixtureBuild(packageId, version, crypto));

            return await this.#store.putLock(Lock.of(root));
        }
        // refuse any other release, as a forge does
        else {
            throw Failure.error({
                code: "NOT_FOUND",
                reason: "RELEASE_NOT_FOUND",
                domain: "@destack/forge",
                message: `release '${named}' of '${packageId}' not found`,
            });
        }
    }

    /** Call the forges of a machine's world for repositories, as the machine does. */
    serve(forges: Pick<Registry, "followed">): void {
        this.#forges = forges;
    }

    /** Follow a release or a tag as a fixture row, and a repository's reference at its forge. */
    async followed(
        packageId: PackageId,
        selection: Exclude<InstallationSelection, { readonly kind: "host" | "checkout" }>,
    ): Promise<ObjectReference> {
        // read a repository's reference at its forge
        if (selection.kind === "repository") {
            return await this.#served().followed(packageId, selection);
        }

        // stand the release or tag for a fixture row of its type
        return selection.kind === "release"
            ? TestRegistry.row("release", selection.version)
            : TestRegistry.row("tag", selection.tag);
    }

    /** Stand a release or a tag of the fixture registry for a row, its identifier naming it. */
    static row(kind: "release" | "tag", name: string): ObjectReference {
        // stand a tag in its forge's home, where a space follows it from
        const id = `${kind}-${TestRegistry.#encode(name)}`;

        return kind === "release"
            ? release.reference(Scope.universe.id, id)
            : tag.reference(FORGE_HOME, id);
    }

    /** Read the version a followed release or tag stands for. */
    async version(_packageId: PackageId, followed: ObjectReference): Promise<string> {
        const named = TestRegistry.#decode(followed.id);

        return TAGS[named] ?? named;
    }

    /** Write a fixture row's name into the free digits of a UUIDv8, so its identifier is valid. */
    static #encode(name: string): string {
        // refuse a name longer than the free digits hold
        const hex = new TextEncoder().encode(name).toHex();
        if (hex.length > FIXTURE_NAME_DIGITS) {
            throw new RangeError(`fixture row name '${name}' is longer than 9 bytes`);
        }

        // place the digits around the version and the variant
        const digits = hex.padEnd(FIXTURE_NAME_DIGITS, "0");

        return `00000000-0000-8${digits.slice(0, 3)}-8${digits.slice(3, 6)}-${digits.slice(6)}`;
    }

    /** Read the name a fixture row's identifier carries. */
    static #decode(id: string): string {
        // take the free digits of its UUID, dropping the padding
        const uuid = id.slice(-36);
        const digits = `${uuid.slice(15, 18)}${uuid.slice(20, 23)}${uuid.slice(24)}`;
        const bytes = Uint8Array.fromHex(digits.replace(/(?:00)+$/u, ""));

        return new TextDecoder().decode(bytes);
    }

    /** Take the forges of the served machine. */
    #served(): Pick<Registry, "followed"> {
        return present(this.#forges, "the forges of a served machine");
    }

    /** The releases its distribution ships: none, as the fixture registry ships none. */
    readonly distribution: readonly PackageRelease[] = [];

    /** Open a release's build, as the universe opens the stacks it applies. */
    async open(packageId: PackageId, named: string): Promise<BuildReader> {
        const lock = await this.#store.lock(await this.find(packageId, named));

        return await this.#store.contents(lock.root);
    }

    /** Release a build locked with the builds it runs with, answering its lock's digest. */
    async release(build: BuildReader, builds: readonly BuildReader[] = []): Promise<Digest> {
        // store the builds and their lock
        const { lock } = await lockBuild(this.#store, build, builds);
        const released = PackageManifest.package(build.manifest);
        this.#pushed.set(`${released.id}@${released.version}`, lock);

        return lock;
    }

    /** Receive a stored lock with the builds it lists, as its forge does. */
    async push(store: Pick<BuildStore, "contents" | "lock">, lock: Digest): Promise<void> {
        // read the lock the registry has
        const isStored = await this.#store.contains(lock);
        const read = isStored ? await this.#store.lock(lock) : await store.lock(lock);
        if (!isStored) {
            for (const build of Lock.builds(read)) {
                if (!(await this.#store.contains(build))) {
                    await this.#store.put(await store.contents(build));
                }
            }
            await this.#store.putLock(read);
        }

        // record the lock under its root's release
        const released = PackageManifest.package((await this.#store.contents(read.root)).manifest);
        this.#pushed.set(`${released.id}@${released.version}`, lock);
    }

    /** Pull a stored lock into a store for some runtimes. */
    async pull(
        store: Pick<BuildStore, "get" | "lock" | "pull">,
        _packageId: PackageId,
        lock: Digest,
        runtimes: readonly Runtime[] | "all",
    ): Promise<Lock> {
        await store.pull(this.#store, { kind: "lock", digest: lock }, runtimes);

        return await store.lock(lock);
    }

    /** Open the stored fixture builds as the store a lock or build is read from. */
    async store(): Promise<Pick<BuildStore, "read">> {
        return this.#store;
    }

    /** The locks of the pushed releases. */
    get locks(): readonly Digest[] {
        return [...this.#pushed.values()];
    }

    /** Report whether the registry has a build by its manifest's digest. */
    contains(manifest: Digest): Promise<boolean> {
        return this.#store.contains(manifest);
    }
}

/** Write a package's fixture build at a version. */
export async function fixtureBuild(
    packageId: PackageId,
    version: string,
    crypto: Crypto,
): Promise<BuildReader> {
    // build the version declaring its archive command and the release's own declarations, of the package's kinds
    const built = await MemoryBuild.of(
        PackageVersion.parse({ id: packageId, name: "@test/package", version }),
        [
            {
                kind: "command",
                kindPackageId: packageId,
                module: "src/command.ts",
                name: "archive",
                description: ARCHIVE,
            },
            ...(DECLARED[version]?.(packageId) ?? []).map((declared) => ({
                description: {},
                kindPackageId: packageId,
                ...declared,
            })),
        ],
        crypto,
        {
            outputs: fixtureOutputs({
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views: {
                        home: {
                            entrypoint: "view.js",
                            stylesheets: [],
                            sandbox: {},
                            presents: [{ packageId, type: "note", priority: "option" }],
                            categories: [],
                            audience: "person",
                            packages: [],
                            render: { mode: "client" },
                        },
                        editor: {
                            entrypoint: "editor.js",
                            stylesheets: [],
                            sandbox: {},
                            presents: [{ packageId, type: "note", priority: "default" }],
                            categories: [],
                            audience: "person",
                            packages: [],
                            render: { mode: "client" },
                        },
                    },
                },
                server: {
                    runtime: "bun",
                    emit: true,
                    workloads: { main: workload },
                    views: {},
                },
            }),
        },
    );

    return built;
}

/** Put a build and the builds it runs with in a store, hoisted into a lock, answering both. */
export async function lockBuild(
    store: Pick<BuildStore, "get" | "put" | "putLock">,
    build: BuildReader,
    builds: readonly BuildReader[],
): Promise<{ readonly lock: Digest; readonly root: Digest }> {
    // put each build by its package
    const locked: Record<string, Digest> = {};
    for (const each of builds) {
        locked[each.manifest.package.name] = await store.put(each);
    }
    const root = await store.put(build);

    // lock them hoisted
    const hoisted = await Lock.hoisted(root, locked, (digest) => store.get(digest));

    return { lock: await store.putLock(hoisted), root };
}

/** Compute the digest of the lock of a build that runs alone. */
export async function lockOf(
    build: Pick<BuildReader, "manifest">,
    crypto: Crypto,
): Promise<Digest> {
    return await Lock.digest(Lock.of(await BuildStore.digest(build.manifest, crypto)), crypto);
}

/** The command every fixture build declares. */
export const ARCHIVE = {
    title: "Archive note",
    packageId: "package-01996ab0-0000-7000-8000-000000000005",
    type: "note",
    method: "archive",
};

/** Evaluate a stack module's export into its stack definition. */
export function evaluateStack(
    module: Readonly<Record<string, unknown>>,
    name: string,
    parameters: Readonly<Record<string, unknown>>,
): JsonObject {
    // require the named export
    const exported = module[name];
    if (exported === undefined) {
        throw StackError.of(
            "DEFINITION_INVALID",
            `stack '${name}' is invalid: the stack package does not export it`,
        );
    }

    // call parameterised definitions with the installation's arguments
    const declared: unknown =
        typeof exported === "function"
            ? Reflect.apply(exported, undefined, [parameters])
            : exported;

    return schema.looseObject({ definition: StackDefinition }).parse(declared).definition;
}
