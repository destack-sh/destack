import { ObjectReference, Subject } from "@destack/sync";
import { defineSchema, schema } from "@destack/schema";
import { PermissionReference } from "../declare/policy.ts";

/** Whether a caller has a permission on one object, and until when that stays true by time alone. */
export const Decision = defineSchema(
    schema.object({
        /** Whether the caller has the permission. */
        isAllowed: schema.boolean(),
        /** The next moment time alone may change the decision, absent when only changes to access do. */
        until: schema.number().int().exactOptional(),
    }),
);
/** Whether a caller has a permission on one object, and until when that stays true by time alone. */
export type Decision = schema.Infer<typeof Decision>;

/** The gate that refuses a request before any grant: credential, elevation, suspension or scope. */
export const Gate = defineSchema(schema.enum(["restricted", "elevation", "suspended", "outside"]));
/** Why a caller's request cannot have a permission on a row at all, before any grant. */
export type Gate = schema.Infer<typeof Gate>;

/** Why one grant fails to admit an authority. */
export const GrantFailure = defineSchema(
    schema.enum([
        "subject",
        "field",
        "role",
        "pending",
        "expired",
        "request",
        "session",
        "linkSecret",
        "assurance",
        "age",
        "delegation",
        "arrow",
    ]),
);
/** Why one grant fails to admit an authority. */
export type GrantFailure = schema.Infer<typeof GrantFailure>;

/** Whether and why a caller has a permission on one object. */
export const Explanation = defineSchema(
    schema.object({
        /** The permission explained. */
        permission: PermissionReference,
        /** The object explained. */
        object: ObjectReference,
        /** Whether the caller has the permission. */
        isAllowed: schema.boolean(),
        /** The gate the request fails before any grant. */
        gate: Gate.exactOptional(),
        /** The represented subject and each delegate, all of which must be admitted. */
        authorities: schema.array(
            schema.object({
                /** The delegate, absent for the represented subject. */
                delegate: Subject.exactOptional(),
                /** The principal the delegate acts for. */
                delegator: Subject.exactOptional(),
                /** Whether a grant admits the authority. */
                isAllowed: schema.boolean(),
                /** Every grant of the permission on the object, with why it fails when it does. */
                grants: schema.array(
                    schema.object({
                        /** How the permission reaches the grant, in order. */
                        path: schema.array(schema.string()),
                        /** The object the relationship, field or role binding sits on. */
                        object: ObjectReference,
                        /** The subject the grant admits. */
                        subject: Subject,
                        /** The bound role, for role bindings. */
                        role: schema.string().exactOptional(),
                        /** Why the grant fails to admit the authority, absent when it admits it. */
                        failure: GrantFailure.exactOptional(),
                    }),
                ),
            }),
        ),
    }),
);
/** Whether and why a caller has a permission on one object. */
export type Explanation = schema.Infer<typeof Explanation>;
