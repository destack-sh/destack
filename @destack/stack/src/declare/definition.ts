import { DeclarationReference } from "@destack/package/declare";
import { graph, type PackageId, type PackageRelease } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { type Digest, type JsonObject, JsonValue, schema } from "@destack/schema";
import { Installation, PackageDescription } from "@destack/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";
import { install, InstallationDefinition } from "./installation.ts";

/** The database every space has, which no stack declares. */
export const MAIN_DATABASE = "main";

/** The collections of a stack definition the stack package reads itself. */
const StackCollections = schema.looseObject({
    /** The databases the stack provisions. */
    databases: schema.jsonObject().default({}),
    /** Independently configured package installations. */
    installations: schema.record(schema.string(), InstallationDefinition).default({}),
    /** The setting values the stack places, by key. */
    settings: schema.jsonObject().default({}),
});

/** A stack definition as JSON, keyed by collection and validated by each object when applied. */
export type StackDefinition = JsonObject;

/** A stack definition as JSON, keyed by collection and validated by each object when applied. */
export const StackDefinition = Object.assign(schema.jsonObject(), {
    /** Read the collections the stack package applies itself. */
    collections(definition: Readonly<Record<string, unknown>>) {
        return StackCollections.parse(definition);
    },

    /** Describe the setting values of a space as the settings a stack places. */
    placed(settings: Readonly<Record<string, JsonValue>>): Record<string, JsonObject> {
        return Object.fromEntries(
            Object.entries(settings).map(([key, value]) => [
                key,
                { ...DeclarationReference.ofKey(key), mode: "set", value },
            ]),
        );
    },
});

/** Read the stack a stack package declares at an export, with its installations' dependencies. */
export async function declaredStack(
    reader: BuildReader,
    name: string,
    settings: Readonly<Record<string, JsonValue>>,
    read: (packageId: PackageId, manifest: Digest) => Promise<BuildReader>,
): Promise<JsonObject> {
    // require the package's stack declared at the export the name has
    const found = (
        await reader.declared(import.meta.destack.package.id, "stack", StackDefinition)
    ).find((entry) => graph.Moniker.name(entry.symbol) === name);
    if (found === undefined) {
        throw StackError.of("STACK_NOT_FOUND", `stack package declares no stack ${name}`);
    }
    const declared = found.description;

    // read the one release the build resolved each Destack package at
    const resolved = oneReleaseEach(await reader.releases(), name);

    // refuse a stack declaring the main database every space has
    const document = StackDefinition.collections(declared);
    if (document.databases[MAIN_DATABASE] !== undefined) {
        throw StackError.of(
            "DEFINITION_INVALID",
            `stack ${name} declares ${MAIN_DATABASE}, but every space already has it`,
        );
    }

    // set each installation to the manifest of the release its package resolved to
    const pinned = Object.fromEntries(
        Object.entries(document.installations).map(([key, installed]) => {
            const manifest = resolved.get(installed.packageId)?.manifest;
            if (manifest === undefined) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `installation ${key} of ${name} installs ${installed.packageId}, which has no release in the stack's build`,
                );
            }

            return [key, { ...installed, selection: { ...installed.selection, manifest } }];
        }),
    );

    // read the installable packages each declared installation's build depends on, except those only its optional capabilities need
    const declaredManifests = new Map(
        Object.values(pinned).map((installed) => [
            installed.packageId,
            installed.selection.manifest,
        ]),
    );
    const closures = await Promise.all(
        Object.values(pinned).map(async (installed) => {
            // read the installation's package and the packages only its optional capabilities need
            const build = await read(installed.packageId, installed.selection.manifest);
            const description = await PackageDescription.of(build);
            const optional = new Set(Installation.optionalPackages(description));

            // refuse a dependency on a declared package at another release
            const releases = await installableReleases(build, read);
            for (const release of releases) {
                const pinnedManifest = declaredManifests.get(release.package.id);
                if (pinnedManifest !== undefined && pinnedManifest !== release.manifest) {
                    throw StackError.of(
                        "DEFINITION_INVALID",
                        `stack ${name} declares ${release.package.id} at another release than ${installed.packageId} depends on`,
                    );
                }
            }

            // list its installable dependencies the stack leaves out
            return releases.filter(
                (release) =>
                    !declaredManifests.has(release.package.id) && !optional.has(release.package.id),
            );
        }),
    );

    // install each once as any package, bound to nothing, at the release its dependents resolved
    const transitive: Record<string, InstallationDefinition> = {};
    for (const release of oneReleaseEach(closures.flat(), name).values()) {
        const installed = install({ ...release.package, resources: {}, secrets: {} }, {});
        if (Object.hasOwn(document.installations, installed.name)) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `stack ${name} declares installation ${installed.name}, which is the key of its dependency ${release.package.name}`,
            );
        }
        transitive[installed.name] = {
            ...installed,
            selection: { ...installed.selection, manifest: release.manifest },
        };
    }

    return StackDefinition.parse({
        ...declared,
        installations: { ...transitive, ...pinned },
        settings: { ...document.settings, ...StackDefinition.placed(settings) },
    });
}

/** List the releases a build depends on that declare a package, leaving libraries out. */
export async function installableReleases(
    reader: BuildReader,
    read: (packageId: PackageId, manifest: Digest) => Promise<BuildReader>,
): Promise<PackageRelease[]> {
    const installable = await Promise.all(
        (await reader.releases()).map(async (release) => {
            const dependency = await read(release.package.id, release.manifest);
            const declared = await PackageDescription.of(dependency);

            return declared === undefined ? [] : [release];
        }),
    );

    return installable.flat();
}

/** Key releases by package, refusing two releases of one package in a stack. */
function oneReleaseEach(
    releases: readonly PackageRelease[],
    stack: string,
): Map<PackageId, PackageRelease> {
    const byPackage = new Map<PackageId, PackageRelease>();
    for (const release of releases) {
        // refuse a second release of a package
        const known = byPackage.get(release.package.id);
        if (known !== undefined && known.manifest !== release.manifest) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `stack ${stack} resolves two releases of ${release.package.name}`,
            );
        }
        byPackage.set(release.package.id, release);
    }

    return byPackage;
}
