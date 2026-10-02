import { defineSchema, schema } from "@destack/schema";
import { PermissionReference } from "../policy/policy.ts";
import type { AccessContext } from "./context.ts";

/** The schema of a restriction: a permission a restricted credential allows, in one scope and optionally on one object. */
const restrictionSchema = defineSchema(
    PermissionReference.extend({
        /** The scope in which the permission applies. */
        scope: schema.string().min(1),
        /** The one object the permission applies to, absent for every object of the scope. */
        objectId: schema.string().min(1).exactOptional(),
    }),
);
/** A permission a restricted credential allows, in one scope and optionally on one object. */
export type Restriction = schema.Infer<typeof restrictionSchema>;

/** A restriction: its schema and the objects it lets a credential reach. */
export const Restriction = {
    /** The schema of a restriction. */
    schema: restrictionSchema,
    allows,
    select,
};

/** Report whether a credential's restrictions allow a permission on an object, as unrestricted credentials do. */
function allows(
    permission: PermissionReference,
    object: { readonly scope: string; readonly id?: string },
    context: AccessContext,
): boolean {
    return (
        context.permissions === undefined ||
        context.permissions.some(
            (entry) =>
                entry.packageId === permission.packageId &&
                entry.type === permission.type &&
                entry.name === permission.name &&
                entry.scope === object.scope &&
                (entry.objectId === undefined ||
                    object.id === undefined ||
                    entry.objectId === object.id),
        )
    );
}

/** List the objects a credential's restrictions select for a permission in a scope, or every object. */
function select(
    permission: PermissionReference,
    scope: string,
    context: AccessContext,
): readonly string[] | "every" {
    // select every object without restrictions
    if (context.permissions === undefined) {
        return "every";
    }

    // collect the objects of the restrictions on the permission in the scope
    const selected: string[] = [];
    for (const entry of context.permissions) {
        if (
            entry.packageId === permission.packageId &&
            entry.type === permission.type &&
            entry.name === permission.name &&
            entry.scope === scope
        ) {
            if (entry.objectId === undefined) {
                return "every";
            }
            selected.push(entry.objectId);
        }
    }

    return selected;
}
