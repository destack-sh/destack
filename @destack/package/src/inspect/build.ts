import { defineSchema, schema } from "@destack/schema";
import { DependencyResolution } from "../package/dependency.ts";
import { PackagePath } from "../file/file.ts";
import { Runtime } from "../runtime/index.ts";

/** A resolved module import in a compiler input or generated file. */
export const BuildImport = defineSchema(
    schema.object({
        /** The input identifier, output path, or external import specifier. */
        path: schema.string().min(1),
        /** Whether the runtime resolves this import. */
        external: schema.boolean(),
        /** Whether the module loads through a dynamic import. */
        dynamic: schema.boolean(),
    }),
);

/** A resolved module import in a compiler input or generated file. */
export type BuildImport = schema.Infer<typeof BuildImport>;

/** A source, virtual, or compiler-generated module. */
export const BuildInput = defineSchema(
    schema.object({
        /** The module's origin. */
        kind: schema.enum(["source", "virtual", "generated"]),
        /** The resolved dependency key; absent for package and virtual modules. */
        package: schema.string().min(1).optional(),
        /** The source path relative to its package, or the virtual module identifier. */
        path: schema.string().min(1),
        /** Resolved static and dynamic imports. */
        imports: schema.array(BuildImport),
        /** Reviewed runtimes; absent when compatibility has not been reviewed. */
        runtimes: schema.array(Runtime).optional(),
        /** Import expressions requiring execution to resolve. */
        unresolvedImports: schema.array(schema.string()).optional(),
    }),
);

/** A source, virtual, or compiler-generated module. */
export type BuildInput = schema.Infer<typeof BuildInput>;

/** A file emitted by the bundler. */
export const BuildOutput = defineSchema(
    schema.object({
        /** Module identifiers assigned to this file by the bundler. */
        inputs: schema.array(schema.string().min(1)),
        /** Static and dynamic imports retained in the generated file. */
        imports: schema.array(BuildImport),
        /** Public JavaScript exports. */
        exports: schema.array(schema.string()),
    }),
);

/** A file emitted by the bundler. */
export type BuildOutput = schema.Infer<typeof BuildOutput>;

/** Resolved packages, parsed modules, and emitted files for one compilation. */
export const BuildDescription = defineSchema(
    schema.object({
        /** Registry releases and local snapshots keyed by resolution identifier. */
        packages: schema.record(schema.string().min(1), DependencyResolution),
        /** Source and generated modules keyed by compiler input identifier. */
        inputs: schema.record(schema.string().min(1), BuildInput),
        /** Generated files keyed by their paths within the package build. */
        outputs: schema.record(PackagePath, BuildOutput),
    }),
);

/** Resolved packages, parsed modules, and emitted files for one compilation. */
export type BuildDescription = schema.Infer<typeof BuildDescription>;
