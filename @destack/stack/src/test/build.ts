import type { Crypto } from "@destack/environment";
import { StackDefinition } from "../declare/index.ts";
import { CallError } from "@destack/object";
import { StackError } from "../error/index.ts";
import { type Digest, type JsonObject, schema } from "@destack/schema";
import { Package, PackageId } from "@destack/package";
import { type BuildReader, OutputContent, type OutputExport } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import type { Registry } from "../server/index.ts";
import type { ReleaseRegistry } from "@destack/account/server";
import { BuildStore } from "@destack/build/store";
import { PackageManifest } from "@destack/package/manifest";

/** The releases the fixture's distribution tags point at, as the forge resolves `<name>@<tag>`. */
export const TAGS: Readonly<Record<string, string>> = { latest: "2026.10.0" };

/** The workload every fixture build's server output has. */
export const workload = {
    entrypoint: ".",
    packages: [],
    triggers: [],
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
            readonly name: string;
            readonly description?: Readonly<Record<string, unknown>>;
        }[]
    >
> = {
    "2026.9.0": () => [{ kind: "relation", name: "editor" }],
    "2026.9.1": () => [
        { kind: "relation", name: "editor" },
        ...["ledger", "archive"].map((name) => ({
            kind: "resource",
            name,
            description: { name, kind: "bucket", spec: {} },
        })),
    ],
    "2026.9.2": () => [
        { kind: "relation", name: "editor" },
        {
            kind: "resource",
            name: "files",
            description: { name: "files", kind: "bucket", spec: {} },
        },
    ],
    "2026.9.3": (packageId) => [
        { kind: "relation", name: "editor" },
        {
            kind: "resource",
            name: "tasks",
            description: { name: "tasks", kind: "installation", spec: { package: packageId } },
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
                dependencies: {},
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

/** The mount the fixture registry serves its builds below. */
const REGISTRY_MOUNT = new URL("http://registry.test/builds/");

/** The registry the fixture's releases come from. */
export class TestRegistry implements Registry, ReleaseRegistry {
    /** The stored fixture builds. */
    readonly #store: BuildStore;
    /** The manifests of the pushed releases, by package and version. */
    readonly #pushed = new Map<string, Digest>();

    /** The fixture app whose declared releases it serves, absent where it serves released ones. */
    readonly #fixture: PackageId | undefined;

    /** Serve the releases released to it from a store, and a fixture app's declared releases. */
    constructor(store: BuildStore, fixture?: PackageId) {
        this.#store = store;
        this.#fixture = fixture;
    }

    /** Find a release at a version or a tag: one released to it, or the fixture app's. */
    async find(packageId: PackageId, release: string): Promise<Digest> {
        // find a release published to the registry
        const version = TAGS[release] ?? release;
        const pushed = this.#pushed.get(`${packageId}@${version}`);
        if (pushed !== undefined) {
            return pushed;
        }
        // serve a declared release of the fixture app, built as its forge stored it
        else if (packageId === this.#fixture && Object.hasOwn(DECLARED, version)) {
            const crypto = this.#store.environment.crypto;

            return this.#store.put(await fixtureBuild(packageId, version, crypto));
        }
        // refuse any other release, as a forge does
        else {
            throw CallError.http("NOT_FOUND", {
                message: `${packageId} has no release ${release}`,
            });
        }
    }

    /** Open a release's build, as the universe opens the stacks it applies. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        return this.#store.contents(await this.find(packageId, release));
    }

    /** Release a build with its npm units, answering its manifest's digest. */
    async release(build: BuildReader, units: readonly BuildReader[] = []): Promise<Digest> {
        // store the units, then the build, and record its manifest under its release
        for (const unit of units) {
            await this.#store.put(unit);
        }
        const manifest = await this.#store.put(build);
        const released = PackageManifest.package(build.manifest);
        this.#pushed.set(`${released.id}@${released.version}`, manifest);

        return manifest;
    }

    /** Receive a stored build with its npm units, as its forge does. */
    async push(store: Pick<BuildStore, "contents" | "units">, manifest: Digest): Promise<Digest> {
        // copy each npm unit, then the build
        const build = await store.contents(manifest);
        const packageId = PackageManifest.package(build.manifest).id;
        for (const unit of await store.units(manifest, packageId)) {
            await this.#store.put(await store.contents(unit));
        }
        await this.#store.put(build);

        // record the build under its release
        const released = PackageManifest.package(build.manifest);
        this.#pushed.set(`${released.id}@${released.version}`, manifest);

        return manifest;
    }

    /** The manifests of the pushed releases. */
    get manifests(): readonly Digest[] {
        return [...this.#pushed.values()];
    }

    /** Report whether the registry has a build by its manifest's digest. */
    contains(manifest: Digest): Promise<boolean> {
        return this.#store.contains(manifest);
    }

    /** Locate a stored fixture build by its manifest's digest, read through the store's fetch. */
    async locate(_packageId: PackageId, manifest: Digest) {
        return {
            location: { manifest, url: new URL(`${manifest}/`, REGISTRY_MOUNT).href },
            fetch: (input: URL, request: RequestInit) =>
                this.#store.fetch(new Request(input.href, request), REGISTRY_MOUNT, {
                    pull: async () => {},
                }),
        };
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
        Package.parse({ id: packageId, name: "@test/package", version }),
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
                ...declared,
                kindPackageId: packageId,
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
                            sandbox: {},
                            presents: [{ packageId, type: "note", priority: "option" }],
                            categories: [],
                            packages: [],
                            render: { mode: "client" },
                        },
                        editor: {
                            entrypoint: "editor.js",
                            sandbox: {},
                            presents: [{ packageId, type: "note", priority: "default" }],
                            categories: [],
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
        throw StackError.of("DEFINITION_INVALID", `stack does not export ${name}`);
    }

    // call parameterised definitions with the installation's arguments
    const declared: unknown =
        typeof exported === "function"
            ? Reflect.apply(exported, undefined, [parameters])
            : exported;

    return schema.looseObject({ definition: StackDefinition }).parse(declared).definition;
}
