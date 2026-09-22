import { defineSchema, schema } from "@destack/schema";
import { ModuleDescription } from "../code/module.ts";

/** Compose library-defined descriptions with the package's module descriptions. */
export function definePackageInspection<Description extends schema.Schema>(
    description: Description,
) {
    return defineSchema(
        schema.object({
            /** The inspection format name. */
            name: schema.string().min(1),
            /** The inspection format version. */
            version: schema.number().int().min(1),
            /** The package's source modules. */
            code: schema.array(ModuleDescription),
            /** Descriptions defined by the package's libraries. */
            descriptions: description,
        }),
    );
}
