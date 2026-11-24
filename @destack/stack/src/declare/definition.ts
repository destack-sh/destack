import { defineKind } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import { type DependencyRelease, graph, type PackageBuild, type PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { type Digest, type JsonObject, JsonValue, schema } from "@destack/schema";
import { ObjectDescription, PackageDescription, SERVER_EXPORT } from "@destack/object";
import type {} from "@destack/package/import-meta";
import { StackError } from "../error/index.ts";
import { InstallationDefinition } from "./installation.ts";

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
    /** The routes the stack declares, by key. */
    routes: schema.jsonObject().default({}),
});

/** A stack definition as JSON, keyed by collection and validated by each object when applied. */
export type StackDefinition = JsonObject;

/** A stack definition as JSON, keyed by collection and validated by each object when applied. */
export const StackDefinition = Object.assign(schema.jsonObject(), {
    /** Read the collections the stack package applies itself. */
    collections(definition: Readonly<Record<string, unknown>>) {
        return StackCollections.parse(definition);
    },

    /** Read the stack a package declares at an export with what its space places in it. */
    async declared(
        reader: BuildReader,
        releases: readonly DependencyRelease[],
        name: string,
        placed: {
            readonly settings: Readonly<Record<string, JsonValue>>;
            readonly routes: Readonly<Record<string, JsonObject>>;
        },
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

        // refuse a stack declaring the main database every space has
        const document = StackDefinition.collections(declared);
        if (document.databases[MAIN_DATABASE] !== undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `stack '${name}' is invalid: it declares '${MAIN_DATABASE}', but every space already has it`,
            );
        }

        // read each package's one build in the stack's image, refusing a second
        const builds = new Map<PackageId, DependencyRelease>();
        for (const entry of releases.filter((each) => each.kind === "destack")) {
            const known = builds.get(entry.package.id);
            if (known !== undefined && known.manifest !== entry.manifest) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `stack '${name}' is invalid: it images two builds of '${entry.package.name}'`,
                );
            }
            builds.set(entry.package.id, entry);
        }

        // refuse an installation of a package without a build in the stack's image, but for one the host provides
        for (const [key, installed] of Object.entries(document.installations)) {
            if (installed.version?.kind !== "host" && !builds.has(installed.packageId)) {
                throw StackError.of(
                    "DEFINITION_INVALID",
                    `installation '${key}' of '${name}' is invalid: it installs a package without a build in the stack's image, so add the package to the stack's dependencies`,
                );
            }
        }

        // run each installation without a version at the release of its build in the stack's image
        const versioned = Object.fromEntries(
            Object.entries(document.installations).map(([key, installed]) => {
                const built = builds.get(installed.packageId);
                const isImaged = installed.version === undefined && built !== undefined;

                return [
                    key,
                    isImaged
                        ? {
                              ...installed,
                              version: { kind: "release", version: built.package.version },
                          }
                        : installed,
                ];
            }),
        );

        return StackDefinition.parse({
            ...declared,
            installations: versioned,
            settings: { ...document.settings, ...StackDefinition.placed(placed.settings) },
            routes: { ...document.routes, ...placed.routes },
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
export const STACK_KIND = defineKind({ name: "stack", description: StackDefinition });

/** List an image's Destack builds but its root whose packages store types in a space. */
export async function installableBuilds(
    entries: Readonly<Record<string, DependencyRelease>>,
    root: PackageId,
    read: (packageId: PackageId, manifest: Digest) => Promise<BuildReader>,
): Promise<PackageBuild[]> {
    // list the image's Destack builds other than its root's
    const destack = Object.values(entries).flatMap((entry) =>
        entry.kind === "destack" && entry.package.id !== root
            ? [{ package: entry.package, manifest: entry.manifest }]
            : [],
    );

    // read each build, keeping the installable ones
    const installable = await Promise.all(
        destack.map(async (built) => {
            // read the build's description, types and server export
            const dependency = await read(built.package.id, built.manifest);
            const declared = await PackageDescription.of(dependency);
            const types = await ObjectDescription.read(dependency);
            const isServing = Object.values(dependency.manifest.outputs).some((output) =>
                Object.hasOwn(output.exports, SERVER_EXPORT),
            );

            // take one that describes itself, serves, and stores a type in a space
            const isInstallable =
                declared !== undefined &&
                isServing &&
                types.some((type) => ObjectDescription.isInSpace(type));

            return isInstallable ? [built] : [];
        }),
    );

    return installable.flat();
}
