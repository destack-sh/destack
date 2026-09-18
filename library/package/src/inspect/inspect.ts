import { defineSchema, schema } from "@destack/schema";
import { ModuleDescription } from "../code/module.ts";

/** Compose library-defined descriptions with the package's module descriptions. */
export function defineInspection<Description extends schema.Schema>(description: Description) {
    return defineSchema(schema.object({
        /** The package's source modules. */
        code: schema.array(ModuleDescription),
        /** Descriptions defined by the package's libraries. */
        descriptions: description,
    }));
}
