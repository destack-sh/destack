import { BucketKind } from "@destack/bucket/declare";
import { InstallationKind } from "@destack/object";
import type { Crypto } from "@destack/environment";
import { StackDefinition } from "@destack/stack/declare";
import { StackError } from "@destack/stack/error";
import { type Digest, type JsonObject, schema, Version } from "@destack/schema";
import { PackageVersion, PackageId } from "@destack/package";
import {
    type BuildReader,
    BuildImage,
    OutputContent,
    type OutputExport,
} from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { BuildStore } from "@destack/build/store";

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

/** What each release of the fixture declares beside its archive action, by release. */
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

/** The fixture package's declared versions, oldest first, as a forge refuses older ones later. */
export const FIXTURE_VERSIONS = Object.keys(DECLARED).toSorted((left, right) =>
    Version.compare(Version.parse(left), Version.parse(right)),
);

/** Write a package's fixture build at a version. */
export async function fixtureBuild(
    packageId: PackageId,
    version: string,
    crypto: Crypto,
    name = "@example/notes",
): Promise<BuildReader> {
    // build the version declaring its archive action and the release's own declarations, of the package's kinds
    const built = await MemoryBuild.of(
        PackageVersion.parse({ id: packageId, name, version }),
        [
            {
                kind: "action",
                kindPackageId: packageId,
                module: "src/action.ts",
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

/** Put a build and the builds it runs with in a store, hoisted into an image, answering both. */
export async function imageBuild(
    store: Pick<BuildStore, "get" | "put" | "putImage">,
    build: BuildReader,
    builds: readonly BuildReader[],
): Promise<{ readonly image: Digest; readonly root: Digest }> {
    // put each build by its package
    const digests: Record<string, Digest> = {};
    for (const each of builds) {
        digests[each.manifest.package.name] = await store.put(each);
    }
    const root = await store.put(build);

    // make an image of them hoisted
    const hoisted = await BuildImage.hoisted(root, digests, (digest) => store.get(digest));

    return { image: await store.putImage(hoisted), root };
}

/** Compute the digest of the image of a build that runs alone. */
export async function imageOf(
    build: Pick<BuildReader, "manifest">,
    crypto: Crypto,
): Promise<Digest> {
    return await BuildImage.digest(
        BuildImage.of(await BuildStore.digest(build.manifest, crypto)),
        crypto,
    );
}

/** The action every fixture build declares. */
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
