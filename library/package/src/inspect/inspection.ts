import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { ModuleGraph } from "../code/graph.ts";
import { definePackageInspection } from "./inspect.ts";

/** The JSON document exchanged with package inspection entrypoints. */
export const PackageInspection = defineSchema(
    definePackageInspection(schema.json()).extend({
        /** The JSON Schema supplied by the package. */
        schema: schema.record(schema.string(), schema.json()),
    }),
);

/** A validated inspection and its JSON Schema. */
export type PackageInspection<Description = unknown> =
    & Omit<schema.Infer<typeof PackageInspection>, "descriptions" | "schema">
    & {
        /** Descriptions produced by the package's libraries. */
        descriptions: Description;
        /** JSON Schema describing the serialized inspection. */
        schema: ReturnType<typeof toJsonSchema>;
    };

/** Validate library descriptions and attach the inspected source modules. */
export function createPackageInspection<Description extends schema.Schema>(
    name: string,
    version: number,
    code: ModuleGraph,
    validator: Description,
    description: schema.Input<Description>,
): PackageInspection<schema.Output<Description>> {
    // validate descriptions against the supplied schema
    name = PackageInspection.shape.name.parse(name);
    version = PackageInspection.shape.version.parse(version);
    const definition = definePackageInspection(validator);
    const modules = [...code.modules.values()];
    const descriptions = validator.parse(description);

    return {
        name,
        version,
        code: modules,
        descriptions,
        schema: toJsonSchema(definition),
    };
}
