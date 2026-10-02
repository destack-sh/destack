import type { ObjectReference, Subject } from "@destack/sync";
import { and, asc, eq, type DatabaseConnection, type Select } from "@destack/db";
import { defineSchema, identifier, schema } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { PermissionReference } from "../policy/policy.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission } from "./table.ts";
import { close } from "./closure.ts";

/** The role every scope's owners have, granting every permission but reserved ones. */
const OWNER_ROLE = {
    name: "owner",
    description: "Every permission in this scope except reserved ones",
};

/** The schema of a role: a named set of permissions a scope defines. */
const roleSchema = defineSchema(
    schema.object({
        /** The role identifier. */
        id: identifier("role"),
        /** The scope defining the role. */
        scope: schema.string().min(1),
        /** The name, unique within the scope. */
        name: schema.string().min(1),
        /** The purpose shown when granting the role. */
        description: schema.string(),
        /** Whether the role grants every permission in its scope but reserved ones. */
        isUniversal: schema.boolean(),
        /** The permissions the role grants itself. */
        permissions: schema.array(PermissionReference),
        /** The revision conditional changes name. */
        revision: schema.number().int().positive(),
    }),
);
/** A named set of permissions a scope defines and relationships bind to subjects. */
export type Role = schema.Infer<typeof roleSchema>;

/** The name, purpose and permissions of a role to define or change. */
export interface RoleRequest {
    /** The name, unique within the scope. */
    readonly name: string;
    /** The purpose shown when granting the role. */
    readonly description: string;
    /** The permissions the role grants. */
    readonly permissions: readonly PermissionReference[];
}

/** A role: its schema, its rows and a scope's owners. */
export const Role = {
    /** The schema of a role. */
    schema: roleSchema,
    read,
    permissions,
    permit,
    replace,
    describe,
    own,
    close,
};

/** Read one role of a scope. */
async function read(
    database: DatabaseConnection,
    scope: string,
    id: string,
): Promise<Select<typeof accessRole>> {
    const [record] = await database
        .select()
        .from(accessRole)
        .where(and(eq(accessRole.scope, scope), eq(accessRole.id, identifier("role").parse(id))));
    if (!record) {
        throw new AccessError("NOT_FOUND", "role not found");
    }

    return record;
}

/** Read the permissions a role grants itself. */
async function permissions(
    database: DatabaseConnection,
    roleId: Select<typeof accessRole>["id"],
): Promise<PermissionReference[]> {
    const rows = await database
        .select()
        .from(accessRolePermission)
        .where(eq(accessRolePermission.roleId, roleId))
        .orderBy(
            asc(accessRolePermission.packageId),
            asc(accessRolePermission.type),
            asc(accessRolePermission.name),
        );

    return rows.map(referenceOf);
}

/** Record the permissions a role grants. */
async function permit(
    database: DatabaseConnection,
    roleId: Select<typeof accessRole>["id"],
    scope: string,
    granted: readonly PermissionReference[],
): Promise<void> {
    if (granted.length > 0) {
        await database.insert(accessRolePermission).values(
            granted.map((permission) => ({
                id: identifier("role-permission").parse(`role-permission-${v7()}`),
                roleId,
                scope,
                packageId: permission.packageId,
                type: permission.type,
                name: permission.name,
            })),
        );
    }
}

/** Replace the permissions a role grants with a declared set. */
async function replace(
    database: DatabaseConnection,
    roleId: Select<typeof accessRole>["id"],
    scope: string,
    granted: readonly PermissionReference[],
): Promise<void> {
    await database.delete(accessRolePermission).where(eq(accessRolePermission.roleId, roleId));
    await permit(database, roleId, scope, granted);
}

/** Describe a role row and its permissions. */
function describe(
    record: Select<typeof accessRole>,
    granted: readonly PermissionReference[],
): Role {
    return {
        id: record.id,
        scope: record.scope,
        name: record.name,
        description: record.description,
        isUniversal: record.isUniversal,
        permissions: [...granted],
        revision: record.revision,
    };
}

/** Define a scope's owner role and bind it to an owner on the scope's object, returning the role. */
async function own(
    database: DatabaseConnection,
    scope: ObjectReference,
    owner: Subject,
    now: number,
): Promise<string> {
    // define the owner role in the scope
    const role = identifier("role").parse(`role-${v7()}`);
    await database.insert(accessRole).values({
        id: role,
        createdAt: now,
        updatedAt: now,
        scope: scope.id,
        ...OWNER_ROLE,
        isUniversal: true,
    });

    // bind it on the scope's object
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: identifier("relationship").parse(`relationship-${v7()}`),
                object: scope,
                role,
                subject: owner,
                createdAt: now,
                expiresAt: null,
            },
            scope.id,
        ),
    );

    return role;
}

/** Read a permission reference from a role permission row. */
function referenceOf(row: Select<typeof accessRolePermission>): PermissionReference {
    return { packageId: row.packageId, type: row.type, name: row.name };
}
