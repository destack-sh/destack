import { z } from "zod";
import { validate } from "../validate/schema.ts";

/** JSON Schema Draft 2020-12 description for inspection and external tooling. */
export type JsonSchema = z.core.JSONSchema.JSONSchema;

/** Check the supported declaration and retain native validation and inference. */
export function defineSchema<Schema extends z.ZodType>(schema: Schema): Schema {
    validate(schema, new Map(), false);

    return schema;
}

/**
 * Describe a schema using JSON Schema Draft 2020-12.
 *
 * Use the declared validator for execution; descriptions can omit format-specific checks.
 */
export function toJsonSchema(schema: z.ZodType): JsonSchema {
    validate(schema, new Map(), false);

    return z.toJSONSchema(schema, {
        target: "draft-2020-12",
        unrepresentable: "throw",
        io: "input",
    });
}
