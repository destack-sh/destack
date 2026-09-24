import { defineSchema, schema } from "@destack/schema";
import { Package } from "./package.ts";
import { PackageError } from "../error/error.ts";

/** Build metadata supplied to a module through import.meta.destack. */
export const ModuleMetadata = defineSchema(
    schema.object({
        /** The package containing this module, including modules bundled from dependencies. */
        package: Package,
    }),
);

/** Immutable module metadata describing its package. */
export type ModuleMetadata = {
    readonly [Key in keyof schema.Infer<typeof ModuleMetadata>]: Readonly<
        schema.Infer<typeof ModuleMetadata>[Key]
    >;
};

/** Require the module metadata the Destack module transform passes to a declaration constructor. */
export function declaringModule(
    module: ModuleMetadata | undefined,
    constructor: string,
): ModuleMetadata {
    // require the argument appended by the module transform
    if (!module) {
        throw new PackageError(
            "INVALID_DEFINITION",
            `${constructor} requires the Destack module transform to supply its package`,
        );
    }

    return ModuleMetadata.parse(module);
}
