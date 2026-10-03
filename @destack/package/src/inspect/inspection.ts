import { defineSchema, type JsonValue, schema } from "@destack/schema";
import { ModuleGraph } from "../code/graph.ts";
import { ModuleDescription } from "../code/module.ts";

/** The schema of the JSON document exchanged with package inspection entrypoints. */
const packageInspectionSchema = defineSchema(
    schema.object({
        /** The inspection format name. */
        name: schema.string().min(1),
        /** The package's source modules. */
        code: schema.array(ModuleDescription),
        /** Descriptions defined by the package's libraries. */
        descriptions: schema.json(),
    }),
);

/** A validated package inspection. */
export type PackageInspection<Description = JsonValue> = Omit<
    schema.Infer<typeof packageInspectionSchema>,
    "descriptions"
> & {
    /** Descriptions produced by the package's libraries. */
    descriptions: Description;
};

/** The JSON document exchanged with package inspection entrypoints. */
export const PackageInspection = Object.assign(packageInspectionSchema, { create });

/** Validate library descriptions and attach the inspected source modules. */
function create<Description extends schema.Schema>(
    name: string,
    code: ModuleGraph,
    validator: Description,
    description: schema.Input<Description>,
): PackageInspection<schema.Output<Description>> {
    // validate descriptions against the supplied schema
    name = packageInspectionSchema.shape.name.parse(name);
    const modules = [...code.modules.values()];
    const descriptions = validator.parse(description);

    return {
        name,
        code: modules,
        descriptions,
    };
}
