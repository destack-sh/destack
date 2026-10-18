import { StackError } from "../error/index.ts";
import { StackDefinition } from "../declare/stack.ts";
import { type JsonObject, schema } from "@destack/schema";
import { Package, PackageId } from "@destack/package";
import { type BuildReader, PackageOutput } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import type { Upgrade } from "@destack/resource";
import type { Registry } from "../server/index.ts";
import { PackageStore } from "@destack/build/store";
import { MemoryBucket } from "@destack/bucket/test";

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

/** The upgrades of the fixture's later releases by release. */
export const UPGRADES: Readonly<Record<string, Upgrade>> = {
    "2026.12.0": { from: "2026.12.0", steps: [] },
    "2026.10.0": {
        from: "2026.9.0",
        steps: [
            {
                action: "create",
                target: "api/procedure/count",
                risk: "safe",
                detail: "add procedure count",
            },
        ],
    },
    "2026.11.0": {
        from: "2026.10.0",
        steps: [
            {
                action: "delete",
                target: "object/note/relation/editor",
                risk: "backward-incompatible",
                detail: "remove: data stored under it no longer applies",
            },
        ],
    },
};

/** Complete a fixture build's outputs with a directory of their own name, exporting nothing. */
export function fixtureOutputs(
    outputs: Readonly<Record<string, Readonly<Record<string, unknown>>>>,
): Readonly<Record<string, PackageOutput>> {
    return Object.fromEntries(
        Object.entries(outputs).map(([name, output]) => [
            name,
            PackageOutput.parse({ directory: name, exports: {}, dependencies: {}, ...output }),
        ]),
    );
}

/** The mount the fixture registry serves its builds below. */
const REGISTRY_MOUNT = new URL("http://registry.test/builds/");

/** The registry the fixture's releases come from. */
export class TestRegistry implements Registry {
    /** The stored fixture builds. */
    readonly #store = new PackageStore(new MemoryBucket());

    /** Store a package's fixture build at a version or at the release a tag points at. */
    async find(packageId: PackageId, release: string): Promise<string> {
        const version = TAGS[release] ?? release;

        return this.#store.put(await fixtureBuild(packageId, version));
    }

    /** Push a build into the registry, answering its manifest's digest. */
    async push(build: BuildReader): Promise<string> {
        return this.#store.put(build);
    }

    /** Report whether the registry keeps a build by its manifest's digest. */
    contains(manifest: string): Promise<boolean> {
        return this.#store.contains(manifest);
    }

    /** Locate a stored fixture build by its manifest's digest, read through the store's fetch. */
    async locate(_packageId: PackageId, manifest: string) {
        return {
            location: { manifest, url: new URL(`${manifest}/`, REGISTRY_MOUNT).href },
            fetch: (input: URL, request: RequestInit) =>
                this.#store.fetch(new Request(input.href, request), REGISTRY_MOUNT, {
                    authorize: async () => ({}),
                }),
        };
    }
}

/** Write a package's fixture build at a version. */
export async function fixtureBuild(packageId: PackageId, version: string): Promise<BuildReader> {
    // build the version with its upgrade
    const upgrade = UPGRADES[version];
    const built = await MemoryBuild.of(
        Package.parse({ id: packageId, name: "@test/package", version }),
        [
            {
                kind: "command",
                kindPackageId: viewPackage,
                module: "src/command.ts",
                name: "archive",
                description: ARCHIVE,
            },
        ],
        {
            ...(upgrade === undefined
                ? {}
                : { upgrade: { package: resourcePackage, description: upgrade } }),
            outputs: fixtureOutputs({
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views: {
                        home: {
                            entrypoint: "view.js",
                            sandbox: {},
                            presents: [],
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

/** The package declaring commands and views. */
const viewPackage = PackageId.parse("package-01a0c80b-614f-73f1-896b-bb411e8814d4");

/** The package defining the upgrade file format. */
const resourcePackage = Package.parse({
    id: "package-01a0c80b-614f-73eb-9845-7906f2b7cfe0",
    name: "@destack/resource",
    version: "2026.9.0",
});

/** Evaluate a stack module's export into its stack definition. */
export function evaluateStack(
    module: Readonly<Record<string, unknown>>,
    name: string,
    parameters: Readonly<Record<string, unknown>>,
): JsonObject {
    // require the named export
    const exported = module[name];
    if (exported === undefined) {
        throw new StackError("INVALID_DEFINITION", `stack does not export ${name}`);
    }

    // call parameterised definitions with the installation's arguments
    const declared: unknown =
        typeof exported === "function"
            ? Reflect.apply(exported, undefined, [parameters])
            : exported;

    return schema.looseObject({ definition: StackDefinition }).parse(declared).definition;
}
