import { ObjectReference, Subject } from "@destack/sync";
import { defineSchema, schema } from "@destack/schema";
import { PermissionReference } from "../declare/policy.ts";

/** Whether a caller has a permission on one object, and until when by time alone. */
export const Decision = defineSchema(
    schema.object({
        /** Whether the caller has the permission. */
        isAllowed: schema.boolean(),
        /** The next moment time alone may change the decision, absent if only access changes do. */
        until: schema.number().int().exactOptional(),
    }),
);
/** Whether a caller has a permission on one object, and until when by time alone. */
export type Decision = schema.Infer<typeof Decision>;

/** The gate that refuses a request before any tuple: grant, elevation, suspension or scope. */
export const Gate = defineSchema(schema.enum(["grant", "elevation", "suspended", "outside"]));
/** Why a caller's request cannot have a permission on a row at all, before any tuple. */
export type Gate = schema.Infer<typeof Gate>;

/** Why one tuple fails to admit the caller. */
export const TupleFailure = defineSchema(
    schema.enum([
        "subject",
        "role",
        "pending",
        "expired",
        "request",
        "credential",
        "linkSecret",
        "assurance",
        "age",
        "arrow",
    ]),
);
/** Why one tuple fails to admit the caller. */
export type TupleFailure = schema.Infer<typeof TupleFailure>;

/** Whether and why a caller has a permission on one object. */
export const Explanation = defineSchema(
    schema.object({
        /** The permission explained. */
        permission: PermissionReference,
        /** The object explained. */
        object: ObjectReference,
        /** Whether the caller has the permission. */
        isAllowed: schema.boolean(),
        /** The gate the request fails before any tuple. */
        gate: Gate.exactOptional(),
        /** Every tuple of the permission on the object, with why it fails when it does. */
        tuples: schema.array(
            schema.object({
                /** How the permission reaches the tuple, in order. */
                path: schema.array(schema.string()),
                /** The object the relationship, field or role binding sits on. */
                object: ObjectReference,
                /** The subject the tuple admits. */
                subject: Subject,
                /** The bound role, for role bindings. */
                role: schema.string().exactOptional(),
                /** Why the tuple fails to admit the caller, absent when it admits it. */
                failure: TupleFailure.exactOptional(),
            }),
        ),
    }),
);
/** Whether and why a caller has a permission on one object. */
export type Explanation = schema.Infer<typeof Explanation>;
