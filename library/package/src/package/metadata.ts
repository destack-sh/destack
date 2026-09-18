import { defineSchema, schema } from "@destack/schema";
import { Package } from "./package.ts";

/** Build metadata supplied to a module through import.meta.destack. */
export const ModuleMetadata = defineSchema(schema.object({
    /** The package containing this module, including modules bundled from dependencies. */
    package: Package,
}));

/** Immutable module metadata describing its package. */
export type ModuleMetadata = {
    readonly [Key in keyof schema.Infer<typeof ModuleMetadata>]: Readonly<
        schema.Infer<typeof ModuleMetadata>[Key]
    >;
};
