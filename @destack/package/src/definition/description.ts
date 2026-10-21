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
        return Object.keys({
            ...requirements.dependencies,
            ...requirements.peerDependencies,
            ...requirements.optionalDependencies,
        });
    },
    /** Decide whether a package runs without a dependency, an optional one or optional peer. */
    isOptional(
        value: Pick<PackageJson, "dependencies" | "optionalDependencies" | "peerDependenciesMeta">,
        name: string,
    ): boolean {
        // accept an optional dependency, which overrides a matching dependency
        if (Object.hasOwn(value.optionalDependencies, name)) {
            return true;
        }

        // accept an optional peer the package does not also require
        const isRequired = Object.hasOwn(value.dependencies, name);
        const isOptionalPeer = value.peerDependenciesMeta[name]?.optional === true;

        return !isRequired && isOptionalPeer;
    },
});

/** A package's package.json, with its destack.json as its definition. */
export type PackageJson = schema.Infer<typeof packageJson>;

/** The runtime dependency groups of a package.json, each absent when it declares none. */
export interface RuntimeRequirements {
    /** The required dependencies. */
    readonly dependencies?: Readonly<Record<string, string>>;
    /** The dependencies a dependent supplies. */
    readonly peerDependencies?: Readonly<Record<string, string>>;
    /** The optional dependencies. */
    readonly optionalDependencies?: Readonly<Record<string, string>>;
}
