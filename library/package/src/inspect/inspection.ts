import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import type { ModuleDescription } from "../code/module.ts";
import { ModuleGraph } from "../code/graph.ts";
import { defineInspection } from "./inspect.ts";

/** The JSON document exchanged with package inspection entrypoints. */
export const InspectionDocument = defineSchema(schema.object({
    /** The inspection format name. */
    name: schema.string().min(1),
    /** The inspection format version. */
    version: schema.number().int().min(1),
    /** Source and library-defined descriptions. */
    description: defineInspection(schema.json()),
    /** The JSON Schema supplied by the package. */
    schema: schema.record(schema.string(), schema.json()),
}));

/** A validated inspection and its JSON Schema. */
export interface Inspection<Description = unknown> {
    /** The inspection format name. */
    name: string;
    /** The inspection format version. */
    version: number;
    /** The inspected source and domain descriptions. */
    description: {
        /** The package's source modules. */
        code: ModuleDescription[];
        /** Descriptions produced by the package's libraries. */
        descriptions: Description;
    };
    /** The schema used to validate the descriptions. */
    schema: ReturnType<typeof toJsonSchema>;
}

/** Validate an inspection and its declaration references. */
export function createInspection<Description extends schema.Schema>(
    name: string,
    version: number,
    code: ModuleGraph,
    validator: Description,
    description: schema.Input<Description>,
): Inspection<schema.Output<Description>> {
    // validate the description and its source references
    schema.string().min(1).parse(name);
    schema.number().int().min(1).parse(version);
    const definition = defineInspection(validator);
    const descriptions = validator.parse(description);
    const modules = [...code.modules.values()];

    return {
        name,
        version,
        description: { code: modules, descriptions },
        schema: toJsonSchema(definition),
    };
}
