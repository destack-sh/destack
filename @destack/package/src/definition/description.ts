import { type JsonObject, type JsonValue, schema } from "@destack/schema";
import { DependencyName, PackageVersion } from "./package.ts";
import { PackageDefinition } from "./definition.ts";

/** Dependency requirements keyed by their imported package names. */
const REQUIREMENTS = schema.record(DependencyName, schema.string().min(1));

/** A package export path, conditional mapping, or fallback list. */
export type PackageExport =
    | string
    | null
    | PackageExport[]
    | {
          [condition: string]: PackageExport;
      };

/** The recursive export format used by package.json. */
export const PackageExport: schema.Schema<PackageExport> = schema.lazy(() =>
    schema.union([
        schema.string().min(1),
        schema.null(),
        schema.array(PackageExport),
        schema.record(schema.string().min(1), PackageExport),
    ]),
);

/** A package's package.json, with its destack.json as its definition. */
const packageJson = schema.object({
    /** The stable package ID, current name, and version. */
    package: PackageVersion,
    /** Destack language, targets, workloads, and compute settings. */
    definition: PackageDefinition,
    /** Authored exports, retaining conditional order and fallback lists. */
    exports: PackageExport.exactOptional(),
    /** The commands the package installs, each its source module by command name. */
    bin: schema.record(schema.string().min(1), schema.string().min(1)),
    /** Required runtime dependencies. */
    dependencies: REQUIREMENTS,
    /** Dependencies supplied by the consuming package. */
    peerDependencies: REQUIREMENTS,
    /** Optional peer declarations keyed by dependency name. */
    peerDependenciesMeta: schema.record(
        DependencyName,
        schema.object({
            /** Whether consumers may omit this peer. */
            optional: schema.boolean().exactOptional(),
        }),
    ),
    /** Optional runtime dependencies, overriding matching dependencies. */
    optionalDependencies: REQUIREMENTS,
    /** Dependencies used to develop and build the package. */
    devDependencies: REQUIREMENTS,
});

/** The start of a range selecting a package of the same workspace, as Bun and pnpm write it. */
const WORKSPACE_RANGE = "workspace:";

/** The tag npm clients install by default, taken for an unlinked workspace dependency. */
const DEFAULT_TAG = "latest";

/** The fields of a package file listing dependencies by range. */
const DEPENDENCY_FIELDS = [
    "dependencies",
    "peerDependencies",
    "optionalDependencies",
    "devDependencies",
] as const;

/** A package's package.json, with its destack.json as its definition. */
export const PackageJson = Object.assign(packageJson, {
    /** List the names a package file requires from its own workspace. */
    workspace(authored: JsonObject): string[] {
        const names = DEPENDENCY_FIELDS.flatMap((field) => {
            const ranges = REQUIREMENTS.safeParse(authored[field]);

            return ranges.success
                ? Object.entries(ranges.data).flatMap(([name, range]) =>
                      range.startsWith(WORKSPACE_RANGE) ? [name] : [],
                  )
                : [];
        });

        return [...new Set(names)].toSorted();
    },

    /** Write a package file as released: each workspace range at its linked version. */
    release(authored: JsonObject, versions: Readonly<Record<string, string>>): JsonObject {
        const released: { [field: string]: JsonValue | undefined } = { ...authored };
        for (const field of DEPENDENCY_FIELDS) {
            const ranges = REQUIREMENTS.safeParse(authored[field]);
            if (ranges.success) {
                released[field] = Object.fromEntries(
                    Object.entries(ranges.data).map(([name, range]) => [
                        name,
                        range.startsWith(WORKSPACE_RANGE) ? (versions[name] ?? DEFAULT_TAG) : range,
                    ]),
                );
            }
        }

        return released;
    },

    /** List the names a package requires at runtime: its dependencies, peers and optional ones. */
    runtimeDependencies(requirements: RuntimeRequirements): string[] {
        const required = Object.keys({
            ...requirements.dependencies,
            ...requirements.optionalDependencies,
        });

        return [...required, ...PackageJson.peers(requirements)];
    },
    /** Read the range each runtime dependency is required at, a metadata-only peer at any. */
    ranges(requirements: RuntimeRequirements): Record<string, string> {
        return Object.fromEntries(
            PackageJson.runtimeDependencies(requirements).map((name) => [
                name,
                requirements.optionalDependencies?.[name] ??
                    requirements.dependencies?.[name] ??
                    requirements.peerDependencies?.[name] ??
                    "*",
            ]),
        );
    },
    /** List the peers a package declares, an optional peer its metadata alone lists among them. */
    peers(requirements: RuntimeRequirements): string[] {
        const required = { ...requirements.dependencies, ...requirements.optionalDependencies };
        const declared = { ...requirements.peerDependencies, ...requirements.peerDependenciesMeta };

        return Object.keys(declared).filter((name) => !Object.hasOwn(required, name));
    },
    /** Decide whether a package runs without a dependency, an optional one or optional peer. */
    isOptional(value: RuntimeRequirements, name: string): boolean {
        // accept an optional dependency, which overrides a matching dependency
        if (value.optionalDependencies?.[name] !== undefined) {
            return true;
        }

        // accept an optional peer the package does not also require
        const isRequired = value.dependencies?.[name] !== undefined;
        const isOptionalPeer = value.peerDependenciesMeta?.[name]?.optional === true;

        return !isRequired && isOptionalPeer;
    },
});

/** A package's package.json, with its destack.json as its definition. */
export type PackageJson = schema.Infer<typeof packageJson>;

/** The runtime dependency groups of a package.json, each absent when it declares none. */
export const RuntimeRequirements = schema.looseObject({
    /** The required dependencies. */
    dependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The dependencies a dependent supplies. */
    peerDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
    /** The peer metadata, which may list an optional peer alone. */
    peerDependenciesMeta: schema
        .record(schema.string(), schema.looseObject({ optional: schema.boolean().exactOptional() }))
        .exactOptional(),
    /** The optional dependencies. */
    optionalDependencies: schema.record(schema.string(), schema.string()).exactOptional(),
});

/** The runtime dependency groups of a package.json, each absent when it declares none. */
export type RuntimeRequirements = schema.Infer<typeof RuntimeRequirements>;
