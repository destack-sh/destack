import { z } from "zod";
import { requireDeclarable } from "../validate/declarable.ts";

/** The JSON Schema Draft 2020-12 description for inspection and external tooling. */
export type JsonSchema = z.core.JSONSchema.JSONSchema;

/** Describe a schema using JSON Schema Draft 2020-12. */
export function toJsonSchema(schema: z.ZodType): JsonSchema {
    requireDeclarable(schema);

    return z.toJSONSchema(schema, {
        target: "draft-2020-12",
        unrepresentable: "throw",
        io: "input",
    });
}

/** Build a validator from a JSON Schema Draft 2020-12 description. */
export function fromJsonSchema(description: JsonSchema): z.ZodType {
    return z.fromJSONSchema(description, { defaultTarget: "draft-2020-12" });
}
