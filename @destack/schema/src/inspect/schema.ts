import { z } from "zod";
import { requireDeclarable } from "../validate/schema.ts";

/** The JSON Schema Draft 2020-12 description for inspection and external tooling. */
export type JsonSchema = z.core.JSONSchema.JSONSchema;

/** Check the supported declaration and retain native validation and inference. */
export function defineSchema<Schema extends z.ZodType>(schema: Schema): Schema {
    requireDeclarable(schema);

    return schema;
}

/** Describe a schema using JSON Schema Draft 2020-12. */
export function toJsonSchema(schema: z.ZodType): JsonSchema {
    requireDeclarable(schema);

    return z.toJSONSchema(schema, {
        target: "draft-2020-12",
        unrepresentable: "throw",
        io: "input",
    });
}
