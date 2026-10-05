import type { z } from "zod";
import { requireDeclarable } from "../validate/declarable.ts";

/** Check the supported declaration and retain native validation and inference. */
export function defineSchema<Schema extends z.ZodType>(schema: Schema): Schema {
    requireDeclarable(schema);

    return schema;
}
