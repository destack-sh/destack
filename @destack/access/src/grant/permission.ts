import { defineSchema, schema } from "@destack/schema";
import { PermissionReference } from "../declare/policy.ts";

/** The schema of a grant permission: a permission in a scope, on an object or through a method. */
const grantPermissionSchema = defineSchema(
    PermissionReference.extend({
        /** The scope in which the permission applies. */
        scope: schema.string().min(1),
        /** The one object the permission applies to, absent for every object of the scope. */
        objectId: schema.string().min(1).exactOptional(),
        /** The one method the permission is used through, absent for every method. */
        method: schema.string().min(1).exactOptional(),
    }),
);
/** A permission a grant gives, in the shape of an RFC 9396 authorization detail. */
export type GrantPermission = schema.Infer<typeof grantPermissionSchema>;

/** A grant permission: its schema and the objects and methods it lets a caller reach. */
export const GrantPermission = {
    /** The schema of a grant permission. */
    schema: grantPermissionSchema,
    allows,
    select,
    intersect,
    reaches,
};

/** Report whether grant permissions allow a permission on an object, all of them when absent. */
function allows(
    permission: PermissionReference,
    object: { readonly scope: string; readonly id?: string },
    permissions: readonly GrantPermission[] | undefined,
): boolean {
    return (
        permissions === undefined ||
        permissions.some(
            (entry) =>
                gives(entry, permission, object.scope) && overlaps(entry.objectId, object.id),
        )
    );
}

/** List the objects grant permissions select for a permission in a scope, or all. */
function select(
    permission: PermissionReference,
    scope: string,
    permissions: readonly GrantPermission[] | undefined,
): readonly string[] | "every" {
    // select every object without grant permissions
    if (permissions === undefined) {
        return "every";
    }

    // collect the objects of the entries giving the permission in the scope
    const selected: string[] = [];
    for (const entry of permissions) {
        if (gives(entry, permission, scope)) {
            if (entry.objectId === undefined) {
                return "every";
            }
            selected.push(entry.objectId);
        }
    }

    return selected;
}

/** Keep the permissions two grants both give, absent when neither limits. */
function intersect(
    left: readonly GrantPermission[] | undefined,
    right: readonly GrantPermission[] | undefined,
): GrantPermission[] | undefined {
    // keep the other side when one side gives everything
    if (left === undefined || right === undefined) {
        return left === undefined ? right?.slice() : left.slice();
    }

    // keep each permission both give, on the narrower object and method
    return left.flatMap((entry) =>
        right
            .filter(
                (other) =>
                    gives(entry, other, other.scope) &&
                    overlaps(entry.objectId, other.objectId) &&
                    overlaps(entry.method, other.method),
            )
            .map((other) => {
                const objectId = entry.objectId ?? other.objectId;
                const method = entry.method ?? other.method;

                return {
                    ...entry,
                    ...(objectId === undefined ? {} : { objectId }),
                    ...(method === undefined ? {} : { method }),
                };
            }),
    );
}

/** Report whether grant permissions reach a method of a type in a scope, binding none or it. */
function reaches(
    type: Pick<PermissionReference, "packageId" | "type">,
    scope: string,
    method: string,
    permissions: readonly GrantPermission[] | undefined,
): boolean {
    // list the entries on the type in the scope
    const entries = (permissions ?? []).filter(
        (entry) =>
            entry.packageId === type.packageId && entry.type === type.type && entry.scope === scope,
    );

    // reach every method of a type no entry binds to methods, else the bound and unbound ones
    return (
        entries.every((entry) => entry.method === undefined) ||
        entries.some((entry) => overlaps(entry.method, method))
    );
}

/** Report whether an entry gives a permission in a scope. */
function gives(entry: GrantPermission, permission: PermissionReference, scope: string): boolean {
    return (
        entry.packageId === permission.packageId &&
        entry.type === permission.type &&
        entry.name === permission.name &&
        entry.scope === scope
    );
}

/** Report whether two selections overlap, an absent one selecting everything. */
function overlaps(left: string | undefined, right: string | undefined): boolean {
    return left === undefined || right === undefined || left === right;
}
