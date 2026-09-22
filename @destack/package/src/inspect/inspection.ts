import { defineSchema, schema } from "@destack/schema";
import { ModuleGraph } from "../code/graph.ts";
import { definePackageInspection } from "./inspect.ts";

/** The JSON document exchanged with package inspection entrypoints. */
export const PackageInspection = defineSchema(definePackageInspection(schema.json()));

/** A validated package inspection. */
export type PackageInspection<Description = unknown> = Omit<
    schema.Infer<typeof PackageInspection>,
    "descriptions"
> & {
    /** Descriptions produced by the package's libraries. */
    descriptions: Description;
};

/** Validate library descriptions and attach the inspected source modules. */
export function createPackageInspection<Description extends schema.Schema>(
    name: string,
    code: ModuleGraph,
    validator: Description,
    description: schema.Input<Description>,
): PackageInspection<schema.Output<Description>> {
    // validate descriptions against the supplied schema
    name = PackageInspection.shape.name.parse(name);
    const modules = [...code.modules.values()];
    const descriptions = validator.parse(description);

    return {
        name,
        code: modules,
        descriptions,
    };
}
