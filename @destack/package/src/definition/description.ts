import { schema } from "@destack/schema";
import { DependencyName, Package } from "./package.ts";
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
    package: Package,
    /** Destack language, targets, workloads, and compute settings. */
    definition: PackageDefinition,
    /** Authored exports, retaining conditional order and fallback lists. */
    exports: PackageExport.exactOptional(),
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

/** A package's package.json, with its destack.json as its definition. */
export const PackageJson = Object.assign(packageJson, {
    /** List the names a package requires at runtime: its dependencies, peers and optional ones. */
    runtimeDependencies(requirements: RuntimeRequirements): string[] {
        const required = Object.keys({
            ...requirements.dependencies,
            ...requirements.optionalDependencies,
        });

        return [...required, ...PackageJson.peers(requirements)];
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
