import { defineDeclarationKind } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { graph, type PackageId, type PackageRelease } from "@destack/package";
import { type BuildReader, PackageManifest } from "@destack/package/manifest";
import { type Digest, type JsonObject, JsonValue, schema } from "@destack/schema";
import {
    Installation,
    ObjectDescription,
    PackageDescription,
    SERVER_EXPORT,
} from "@destack/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";
import { InstallationDefinition } from "./installation.ts";
import { BOOTSTRAP_PACKAGES } from "./bootstrap.ts";

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

    /** Read the stack a package declares at an export, with its installations' dependencies. */
    async declared(
        reader: BuildReader,
        name: string,
        settings: Readonly<Record<string, JsonValue>>,
        read: (packageId: PackageId, manifest: Digest) => Promise<BuildReader>,
    ): Promise<JsonObject> {
        // require the package's stack declared at the export the name has
        const found = (await STACK_KIND.read(reader)).find(
            (entry) => graph.Moniker.name(entry.symbol) === name,
        );
        if (found === undefined) {
            throw StackError.of(
                "STACK_NOT_FOUND",
                `stack '${name}' in the stack package not found`,
            );
        }
        const declared = found.description;

        // read the one release the build resolved each Destack package at
        const resolved = oneReleaseEach(await reader.releases(), name);

        // refuse a stack declaring the main database every space has
        const document = StackDefinition.collections(declared);
        if (document.databases[MAIN_DATABASE] !== undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `stack '${name}' is invalid: it declares '${MAIN_DATABASE}', but every space already has it`,
            );
        }

        // read each installation with the manifest of the release its package resolved to
        const declaredInstallations = Object.entries(document.installations).map(
            ([key, installed]) => {
                const manifest = resolved.get(installed.packageId)?.manifest;
                if (manifest === undefined) {
                    throw StackError.of(
                        "DEFINITION_INVALID",
                        `installation '${key}' of '${name}' is invalid: it installs a package without a release in the stack's build, so add the package to the stack's dependencies`,
                    );
                }

                return { key, installed, manifest };
            },
        );

        // pin each release selection to its manifest, leaving a tag selection to follow its tag
        const pinned = Object.fromEntries(
            declaredInstallations.map(({ key, installed, manifest }) => [
                key,
                installed.selection.kind === "release"
                    ? { ...installed, selection: { ...installed.selection, manifest } }
                    : installed,
            ]),
        );

        // read the installable packages each declared installation's build depends on, except those only its optional capabilities need
        const own = new Set(Object.values(BOOTSTRAP_PACKAGES).map((handle) => handle.id));
        for (const { key, installed, manifest } of declaredInstallations) {
            // read the installation's package and the packages only its optional capabilities need
            const build = await read(installed.packageId, manifest);
            const description = await PackageDescription.of(build);
            const optional = new Set(Installation.optionalPackages(description));
            const releases = (await installableReleases(build, read)).filter(
                (release) => !optional.has(release.package.id) && !own.has(release.package.id),
            );

            // require each dependency declared at the release it resolved, in the store with a store-run dependent
            for (const release of releases) {
                const dependency = declaredInstallations.find(
                    (each) => each.installed.packageId === release.package.id,
                );
                if (dependency === undefined) {
                    throw StackError.of(
                        "DEFINITION_INVALID",
                        `installation '${key}' of '${name}' is invalid: it needs '${release.package.name}', so declare an installation of it`,
                    );
                } else if (dependency.manifest !== release.manifest) {
                    throw StackError.of(
                        "DEFINITION_INVALID",
                        `stack '${name}' is invalid: it declares '${release.package.name}' at another release than '${PackageManifest.package(build.manifest).name}' depends on`,
                    );
                } else if (
                    installed.isolation === "host" &&
                    dependency.installed.isolation !== "host"
                ) {
                    throw StackError.of(
                        "DEFINITION_INVALID",
                        `installation '${key}' of '${name}' is invalid: it runs in the store, so its dependency '${dependency.key}' must too and declares host isolation`,
                    );
                }
            }
        }

        return StackDefinition.parse({
            ...declared,
            installations: pinned,
            settings: { ...document.settings, ...StackDefinition.placed(settings) },
        });
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

/** The stack declaration kind, which stack defines. */
export const STACK_KIND = defineDeclarationKind("stack", StackDefinition);

/** List the releases a build depends on that declare a package with a type stored in a space. */
export async function installableReleases(
    reader: BuildReader,
    read: (packageId: PackageId, manifest: Digest) => Promise<BuildReader>,
): Promise<PackageRelease[]> {
    const installable = await Promise.all(
        (await reader.releases()).map(async (release) => {
            // take a dependency serving its own package with a type stored in a space, leaving a contract another package implements
            const dependency = await read(release.package.id, release.manifest);
            const declared = await PackageDescription.of(dependency);
            const types = await ObjectDescription.read(dependency);
            const isServing = Object.values(dependency.manifest.outputs).some((output) =>
                Object.hasOwn(output.exports, SERVER_EXPORT),
            );
            const isInstallable =
                declared !== undefined &&
                isServing &&
                types.some((type) => ObjectDescription.isInSpace(type));

            return isInstallable ? [release] : [];
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
                `stack '${stack}' is invalid: it resolves two releases of '${release.package.name}'`,
            );
        }
        byPackage.set(release.package.id, release);
    }

    return byPackage;
}
