import type { Crypto } from "@destack/environment";
import { StackDefinition } from "../declare/index.ts";
import { StackError } from "../error/index.ts";
import { type Digest, type JsonObject, schema } from "@destack/schema";
import { Package, PackageId } from "@destack/package";
import { type BuildReader, type OutputExport, PackageOutput } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import type { Registry } from "../server/index.ts";
import type { ReleaseRegistry } from "@destack/account/server";
import { PackageStore, RUNTIME_REFERRER } from "@destack/build/store";
import { Runtime } from "@destack/package/runtime";
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
    Record<string, readonly { readonly kind: string; readonly name: string }[]>
> = {
    "2026.9.0": [{ kind: "relation", name: "editor" }],
    "2026.10.0": [{ kind: "relation", name: "editor" }],
    "2026.11.0": [{ kind: "procedure", name: "count" }],
    "2026.12.0": [{ kind: "procedure", name: "count" }],
};

/** Complete fixture build outputs with their directory, exporting each workload's entrypoint. */
export function fixtureOutputs(
    outputs: Readonly<Record<string, Readonly<Record<string, unknown>>>>,
): Readonly<Record<string, PackageOutput>> {
    return Object.fromEntries(
        Object.entries(outputs).map(([name, output]) => {
            // export each workload's entrypoint as a module importing no dependency, as a build does
            const parsed = PackageOutput.parse({
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
                PackageOutput.parse({
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
    readonly #store: PackageStore;
    /** The manifests of the pushed releases, by package and version. */
    readonly #pushed = new Map<string, Digest>();

    /** Serve the releases from a store: one in memory, or the run's build store. */
    constructor(store: PackageStore) {
        this.#store = store;
    }

    /** Find a pushed release, else store a fixture build at a version or at a tag's release. */
    async find(packageId: PackageId, release: string): Promise<Digest> {
        const version = TAGS[release] ?? release;

        return (
            this.#pushed.get(`${packageId}@${version}`) ??
            this.#publish(await fixtureBuild(packageId, version, this.#store.environment.crypto))
        );
    }

    /** Open a release's build, as the universe opens the stacks it applies. */
    async open(packageId: PackageId, release: string): Promise<BuildReader> {
        return this.#store.contents(await this.find(packageId, release));
    }

    /** Release a build with its npm units, answering its manifest's digest. */
    async release(build: BuildReader, units: readonly BuildReader[] = []): Promise<Digest> {
        // store the units, then the build, and record its manifest under its release
        for (const unit of units) {
            await this.#publish(unit);
        }
        const manifest = await this.#publish(build);
        const released = PackageManifest.package(build.manifest);
        this.#pushed.set(`${released.id}@${released.version}`, manifest);

        return manifest;
    }

    /** Receive a stored build with its npm units, deriving its runtime builds as its forge does. */
    async push(store: Pick<PackageStore, "contents" | "units">, manifest: Digest): Promise<Digest> {
        // copy each npm unit, then the build with its runtime builds
        const build = await store.contents(manifest);
        const packageId = PackageManifest.package(build.manifest).id;
        for (const unit of await store.units(manifest, packageId)) {
            await this.#publish(await store.contents(unit));
        }
        await this.#publish(build);

        // record the build under its release
        const released = PackageManifest.package(build.manifest);
        this.#pushed.set(`${released.id}@${released.version}`, manifest);

        return manifest;
    }

    /** Store a build with the runtime build of each emitted output, as its forge publishes it. */
    async #publish(build: BuildReader): Promise<Digest> {
        const manifest = await this.#store.put(build);
        await this.#store.attachRuntimes(manifest);

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

    /** Read a fixture build's attachment at a path, deriving runtime builds as a forge does. */
    async referrer(_packageId: PackageId, manifest: Digest, path: string) {
        const [prefix, runtime] = path.split("/");
        const parsed = Runtime.safeParse(runtime);

        return prefix === RUNTIME_REFERRER && parsed.success
            ? await this.#store.attachRuntime(manifest, parsed.data)
            : (await this.#store.referrer(manifest, path))?.digest;
    }

    /** Locate a stored fixture build by its manifest's digest, read through the store's fetch. */
    async locate(_packageId: PackageId, manifest: Digest) {
        return {
            location: { manifest, url: new URL(`${manifest}/`, REGISTRY_MOUNT).href },
            fetch: (input: URL, request: RequestInit) =>
                this.#store.fetch(new Request(input.href, request), REGISTRY_MOUNT, {
                    authorize: async () => {},
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
            ...(DECLARED[version] ?? []).map((declared) => ({
                ...declared,
                kindPackageId: packageId,
                description: {},
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
                            packages: [],
                        },
                        editor: {
                            entrypoint: "editor.js",
                            sandbox: {},
                            presents: [{ packageId, type: "note", priority: "default" }],
                            packages: [],
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
