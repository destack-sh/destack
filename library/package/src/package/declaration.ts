import { defineSchema, schema } from "@destack/schema";
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

/** Package declarations combined from package.json and destack.json. */
export const PackageDeclaration = defineSchema(
    schema.object({
        /** The stable package ID, current name, and version. */
        package: Package,
        /** Destack language, targets, workloads, and compute settings. */
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
    }),
);

/** Package declarations consumed by build and registry tooling. */
export type PackageDeclaration = schema.Infer<typeof PackageDeclaration>;
