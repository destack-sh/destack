import { schema } from "@destack/schema";
import { PermissionReference } from "../declare/policy.ts";

/** How a grant permission's uses run: at once, or each once its delegator approves the call. */
export const GRANT_MODES = ["allowed", "approval"] as const;

/** How a grant permission's uses run. */
export type GrantMode = (typeof GRANT_MODES)[number];

/** The schema of a grant permission: a permission in a scope, on an object or through a method. */
const grantPermissionSchema = PermissionReference.extend({
    /** The scope the permission applies in, absent for every scope of the delegator. */
    scope: schema.string().min(1).exactOptional(),
    /** The one object the permission applies to, absent for every object of the scope. */
    objectId: schema.string().min(1).exactOptional(),
    /** The one method the permission is used through, absent for every method. */
    method: schema.string().min(1).exactOptional(),
    /** How each use runs: at once by default, or once its delegator approves the call. */
    mode: schema.enum(GRANT_MODES).exactOptional(),
});
/** A permission a grant gives, in the shape of an RFC 9396 authorization detail. */
export type GrantPermission = schema.Infer<typeof grantPermissionSchema>;

/** A grant permission: its schema and the objects and methods it allows. */
export const GrantPermission = {
    /** The schema of a grant permission. */
    schema: grantPermissionSchema,

    /** Report whether grant permissions allow a permission on an object, all when absent. */
    allows(
        permission: PermissionReference,
        object: { readonly scope: string; readonly id?: string },
        permissions: readonly GrantPermission[] | undefined,
    ): boolean {
        return (
            permissions === undefined ||
            permissions.some(
                (entry) =>
                    GrantPermission.gives(entry, permission, object.scope) &&
                    GrantPermission.overlaps(entry.objectId, object.id),
            )
        );
    },

    /** List the objects grant permissions select for a permission in a scope, or all. */
    select(
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
            if (GrantPermission.gives(entry, permission, scope)) {
                if (entry.objectId === undefined) {
                    return "every";
                }
                selected.push(entry.objectId);
            }
        }

        return selected;
    },

    /** Intersect the permissions two grants give, absent when neither limits. */
    intersect(
        left: readonly GrantPermission[] | undefined,
        right: readonly GrantPermission[] | undefined,
    ): GrantPermission[] | undefined {
        // take the other side when one side gives everything
        if (left === undefined || right === undefined) {
            return left === undefined ? right?.slice() : left.slice();
        }

        // take each permission both give, in the narrower scope, object and method
        return left.flatMap((entry) =>
            right
                .filter(
                    (other) =>
                        PermissionReference.key(entry) === PermissionReference.key(other) &&
                        GrantPermission.overlaps(entry.scope, other.scope) &&
                        GrantPermission.overlaps(entry.objectId, other.objectId) &&
                        GrantPermission.overlaps(entry.method, other.method),
                )
                .map((other) => {
                    // take the narrower of each selection, and approval when either side asks for it
                    const scope = entry.scope ?? other.scope;
                    const objectId = entry.objectId ?? other.objectId;
                    const method = entry.method ?? other.method;
                    const isOnApproval =
                        GrantPermission.isOnApproval(entry) || GrantPermission.isOnApproval(other);

                    return {
                        ...entry,
                        ...(scope === undefined ? {} : { scope }),
                        ...(objectId === undefined ? {} : { objectId }),
                        ...(method === undefined ? {} : { method }),
                        ...(isOnApproval ? { mode: "approval" as const } : {}),
                    };
                }),
        );
    },

    /** Report whether grant permissions allow a method of a type in a scope. */
    allowsMethod(
        type: Pick<PermissionReference, "packageId" | "type">,
        scope: string,
        method: string,
        permissions: readonly GrantPermission[] | undefined,
    ): boolean {
        // list the entries on the type in the scope
        const entries = (permissions ?? []).filter(
            (entry) =>
                entry.packageId === type.packageId &&
                entry.type === type.type &&
                GrantPermission.overlaps(entry.scope, scope),
        );

        // allow every method of a type when no entry binds methods, else the bound and unbound ones
        return (
            entries.every((entry) => entry.method === undefined) ||
            entries.some((entry) => GrantPermission.overlaps(entry.method, method))
        );
    },

    /** Report whether grant permissions give a permission on an object, each use once approved. */
    allowsOnApproval(
        permission: PermissionReference,
        object: { readonly scope: string; readonly id?: string },
        permissions: readonly GrantPermission[] | undefined,
    ): boolean {
        return (permissions ?? []).some(
            (entry) =>
                GrantPermission.isOnApproval(entry) &&
                GrantPermission.covers(entry, permission, object.scope) &&
                GrantPermission.overlaps(entry.objectId, object.id),
        );
    },

    /** Report whether an entry's uses each wait for its delegator's approval. */
    isOnApproval(entry: GrantPermission): boolean {
        return entry.mode === "approval";
    },

    /** Report whether an entry gives a permission in a scope at once, without approval. */
    gives(entry: GrantPermission, permission: PermissionReference, scope: string): boolean {
        return (
            !GrantPermission.isOnApproval(entry) && GrantPermission.covers(entry, permission, scope)
        );
    },

    /** Report whether an entry has a permission in a scope, whatever its mode. */
    covers(entry: GrantPermission, permission: PermissionReference, scope: string): boolean {
        return (
            entry.packageId === permission.packageId &&
            entry.type === permission.type &&
            entry.name === permission.name &&
            GrantPermission.overlaps(entry.scope, scope)
        );
    },

    /** Report whether two selections overlap, an absent one selecting everything. */
    overlaps(left: string | undefined, right: string | undefined): boolean {
        return left === undefined || right === undefined || left === right;
    },
};
