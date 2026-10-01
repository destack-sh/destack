import { defineSchema, schema } from "@destack/schema";
import { Package } from "./package.ts";
import { PackageError } from "../error/error.ts";

/** The schema of the build metadata supplied to a module through import.meta.destack. */
const moduleMetadataSchema = defineSchema(
    schema.object({
        /** The package containing this module, including modules bundled from dependencies. */
        package: Package,
    }),
);

/** Immutable module metadata describing its package. */
export type ModuleMetadata = {
    readonly [Key in keyof schema.Infer<typeof moduleMetadataSchema>]: Readonly<
        schema.Infer<typeof moduleMetadataSchema>[Key]
    >;
};

/** Build metadata supplied to a module through import.meta.destack. */
export const ModuleMetadata = Object.assign(moduleMetadataSchema, {
    /** Require the module metadata the Destack module transform passes to a declaration constructor. */
    require(module: ModuleMetadata | undefined, constructor: string): ModuleMetadata {
        // require the argument appended by the module transform
        if (!module) {
            throw new PackageError(
                "INVALID_DEFINITION",
                `${constructor} requires the Destack module transform to supply its package`,
            );
        }

        return moduleMetadataSchema.parse(module);
    },
});
