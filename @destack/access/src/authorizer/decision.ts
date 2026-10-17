import { ObjectReference, Subject } from "@destack/sync";
import { schema } from "@destack/schema";
import { PermissionReference } from "../declare/policy.ts";

/** Whether a caller has a permission on one object, and until when. */
export const Decision = schema.object({
    /** Whether the caller has the permission. */
    isAllowed: schema.boolean(),
    /** The next time the decision may change on its own, absent if only access changes can. */
    until: schema.number().int().exactOptional(),
});
/** Whether a caller has a permission on one object, and until when. */
export type Decision = schema.Infer<typeof Decision>;

/** The gate that refuses a request before any tuple: grant, elevation, suspension or scope. */
export const Gate = schema.enum(["grant", "elevation", "suspended", "outside"]);
/** Why a caller's request cannot have a permission on a row at all, before any tuple. */
export type Gate = schema.Infer<typeof Gate>;

/** Why one tuple fails to allow the caller. */
export const TupleFailure = schema.enum([
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
]);
/** Why one tuple fails to allow the caller. */
export type TupleFailure = schema.Infer<typeof TupleFailure>;

/** Whether and why a caller has a permission on one object. */
export const Explanation = schema.object({
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
            /** The steps from the permission to the tuple, in order. */
            path: schema.array(schema.string()),
            /** The object the relationship, field or role binding sits on. */
            object: ObjectReference,
            /** The subject the tuple allows. */
            subject: Subject,
            /** The bound role, for role bindings. */
            role: schema.string().exactOptional(),
            /** Why the tuple fails to allow the caller, absent when it allows it. */
            failure: TupleFailure.exactOptional(),
        }),
    ),
});
/** Whether and why a caller has a permission on one object. */
export type Explanation = schema.Infer<typeof Explanation>;
