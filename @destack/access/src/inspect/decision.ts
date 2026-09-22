import { defineSchema, schema } from "@destack/schema";
import { ObjectReference, PermissionReference } from "../access/object.ts";

/** An authorization result with evaluated grants and its authoritative revision. */
export const DecisionDescription = defineSchema(
    schema.object({
        permission: PermissionReference,
        object: ObjectReference,
        allowed: schema.boolean(),
        grants: schema.array(schema.string().min(1)),
        revision: schema.union([schema.string(), schema.number().finite()]),
    }),
);
