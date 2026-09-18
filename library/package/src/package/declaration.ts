import { defineSchema, schema } from "@destack/schema";
import { DependencyName, Package } from "./package.ts";
import { PackageDefinition } from "./definition.ts";
import { PackageError } from "../error/index.ts";

/** Dependency requirements keyed by their imported package names. */
const REQUIREMENTS = schema.record(DependencyName, schema.string().min(1));

/** A package export path, conditional mapping, or fallback list. */
export type PackageExport = string | null | PackageExport[] | {
    [condition: string]: PackageExport;
};

/** The recursive export format used by package.json. */
export const PackageExport: schema.Schema<PackageExport> = schema.lazy(() =>
    schema.union([
        schema.string().min(1),
        schema.null(),
        schema.array(PackageExport),
        schema.record(schema.string().min(1), PackageExport),
    ])
);

/** Package declarations combined from package.json and destack.json. */
export const PackageDeclaration = defineSchema(schema.object({
    /** The package name and version. */
    package: Package,
    /** Destack language, target, and inspection declarations. */
    definition: PackageDefinition,
    /** Authored exports, retaining conditional order and fallback lists. */
    exports: PackageExport.optional(),
    /** Required runtime dependencies. */
    dependencies: REQUIREMENTS,
    /** Dependencies supplied by the consuming package. */
    peerDependencies: REQUIREMENTS,
    /** Optional peer declarations keyed by dependency name. */
    peerDependenciesMeta: schema.record(
        DependencyName,
        schema.object({
            /** Whether consumers may omit this peer. */
            optional: schema.boolean().optional(),
        }),
    ),
    /** Optional runtime dependencies, overriding matching dependencies. */
    optionalDependencies: REQUIREMENTS,
    /** Dependencies used to develop and build the package. */
    devDependencies: REQUIREMENTS,
}));

/** Package declarations consumed by build and registry tooling. */
export type PackageDeclaration = schema.Infer<typeof PackageDeclaration>;

/** Combine authored manifests without duplicating their fields on disk. */
export function parseDeclaration(metadata: unknown, definition: unknown): PackageDeclaration {
    const fields = schema.record(schema.string(), schema.json()).parse(metadata);

    const declaration = PackageDeclaration.parse({
        package: { name: fields.name, version: fields.version },
        definition,
        exports: fields.exports,
        dependencies: fields.dependencies === undefined ? {} : fields.dependencies,
        peerDependencies: fields.peerDependencies === undefined ? {} : fields.peerDependencies,
        peerDependenciesMeta: fields.peerDependenciesMeta === undefined
            ? {}
            : fields.peerDependenciesMeta,
        optionalDependencies: fields.optionalDependencies === undefined
            ? {}
            : fields.optionalDependencies,
        devDependencies: fields.devDependencies === undefined ? {} : fields.devDependencies,
    });

    // associate peer metadata with a declared requirement
    for (const name of Object.keys(declaration.peerDependenciesMeta)) {
        if (!Object.hasOwn(declaration.peerDependencies, name)) {
            throw new PackageError("INVALID_DEPENDENCY", `Undeclared peer dependency: ${name}`);
        }
    }

    return declaration;
}
