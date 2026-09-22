import {
    and,
    eq,
    exists,
    gt,
    isNull,
    or,
    type Column,
    type DatabaseConnection,
    type SQL,
    type Table,
} from "@destack/db";
import type { PermissionReference } from "../access/object.ts";

/** Compile scoped role grants against authoritative binding and permission tables. */
export class RoleQuery {
    /** Authoritative role and permission columns supplied by the model package. */
    readonly mapping: RoleMapping;

    /** Retain a table mapping without reading or caching permission records. */
    constructor(mapping: RoleMapping) {
        this.mapping = Object.freeze({ ...mapping });
    }

    /** Select an active scoped binding granting the requested object permission. */
    where(
        database: DatabaseConnection,
        permission: PermissionReference,
        selection: RoleSelection,
    ): SQL {
        const mapping = this.mapping;
        const grants = database
            .select({ role: mapping.permissionRole })
            .from(mapping.bindingTable)
            .innerJoin(mapping.permissionTable, eq(mapping.permissionRole, mapping.bindingRole))
            .where(
                and(
                    selection.scope,
                    selection.subject,
                    isNull(mapping.revokedAt),
                    or(isNull(mapping.expiresAt), gt(mapping.expiresAt, selection.now)),
                    permissionPredicate(mapping, permission, selection.object),
                ),
            );

        return exists(grants);
    }
}

/** Application-selected scope, recipient and target under one authorization read time. */
export interface RoleSelection {
    /** Exact authority scope, including exclusions for nested scopes. */
    readonly scope: SQL;
    /** Verified direct or group subject selection. */
    readonly subject: SQL;
    /** Exact target; omission requires a scope-wide grant. */
    readonly object?: string | Column;
    /** Time used consistently for expiry checks. */
    readonly now: number;
}

/** Columns shared by role permissions and restricted credentials. */
export interface PermissionMapping {
    /** Stable declaring package identifier. */
    readonly packageId: Column;
    /** Protected object type name. */
    readonly type: Column;
    /** Permission name. */
    readonly name: Column;
    /** Optional restriction to one protected object. */
    readonly objectId: Column;
}

/** Role grant tables declared by the domain model. */
export interface RoleMapping extends PermissionMapping {
    /** Scoped role bindings. */
    readonly bindingTable: Table;
    /** Permissions belonging to each role. */
    readonly permissionTable: Table;
    /** Role identifier in the binding. */
    readonly bindingRole: Column;
    /** Role identifier in the permission. */
    readonly permissionRole: Column;
    /** Binding revocation time. */
    readonly revokedAt: Column;
    /** Binding expiry time. */
    readonly expiresAt: Column;
}

/** Match a permission without expanding an object grant into scope-wide authority. */
export function permissionPredicate(
    mapping: PermissionMapping,
    permission: PermissionReference,
    object?: string | Column,
): SQL {
    return and(
        eq(mapping.packageId, permission.packageId),
        eq(mapping.type, permission.type),
        eq(mapping.name, permission.name),
        object === undefined
            ? isNull(mapping.objectId)
            : or(isNull(mapping.objectId), eq(mapping.objectId, object)),
    )!;
}
