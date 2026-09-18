import { z } from "zod";
import { validate } from "../validate/schema.ts";

/** JSON Schema Draft 2020-12 describing a portable Destack value. */
export type JsonSchema = z.core.JSONSchema.JSONSchema;

/** Check portability and retain the schema's native validation and inference. */
export function defineSchema<T extends z.ZodType>(schema: T): T {
    toJsonSchema(schema);

    return schema;
}

/** Describe the accepted JSON values using JSON Schema Draft 2020-12. */
export function toJsonSchema(schema: z.ZodType): JsonSchema {
    validate(schema, new Map(), false);

    return z.toJSONSchema(schema, {
        target: "draft-2020-12",
        unrepresentable: "throw",
        io: "input",
    });
}
