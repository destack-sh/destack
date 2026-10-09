import type { ObjectReference, Subject } from "@destack/sync";
import { and, asc, eq, type DatabaseConnection, type Select } from "@destack/db";
import { defineSchema, schema } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import { PermissionReference } from "../declare/policy.ts";
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
        id: schema.identifier("role"),
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
export const RoleRequest = defineSchema(
    schema.object({
        /** The name, unique within the scope. */
        name: schema.string().min(1).max(200),
        /** The purpose shown when granting the role. */
        description: schema.string().max(1000),
        /** The permissions the role grants. */
        permissions: schema.array(PermissionReference),
    }),
);
/** The name, purpose and permissions of a role to define or change. */
export type RoleRequest = schema.Infer<typeof RoleRequest>;

/** A role: its schema and the reads of its rows, with writes going through `Authorization`. */
export const Role = {
    /** The schema of a role. */
    schema: roleSchema,
    read,
    permissions,
    describe,
    close,
    keepOwner,
    keepInherent,
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
        .where(
            and(
                eq(accessRole.scope, scope),
                eq(accessRole.id, schema.identifier("role").parse(id)),
            ),
        );
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

/** Define a role under a name free in its scope with its permissions and manager, absent when another role holds the name. */
export async function define(
    database: DatabaseConnection,
    scope: string,
    request: RoleRequest,
    now: number,
    manager: Manager | null,
): Promise<Select<typeof accessRole> | undefined> {
    // insert the role under a name free in the scope
    const [record] = await database
        .insert(accessRole)
        .values({
            id: schema.identifier("role").parse(`role-${v7()}`),
            createdAt: now,
            updatedAt: now,
            scope,
            name: request.name,
            description: request.description,
            ...Manager.values(manager),
        })
        .onConflictDoNothing()
        .returning();

    // record its permissions
    if (record !== undefined) {
        await permit(database, record.id, scope, request.permissions);
    }

    return record;
}

/** Record the permissions a role grants. */
export async function permit(
    database: DatabaseConnection,
    roleId: Select<typeof accessRole>["id"],
    scope: string,
    granted: readonly PermissionReference[],
): Promise<void> {
    if (granted.length > 0) {
        await database.insert(accessRolePermission).values(
            granted.map((permission) => ({
                id: schema.identifier("role-permission").parse(`role-permission-${v7()}`),
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
export async function replace(
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
export async function own(
    database: DatabaseConnection,
    scope: ObjectReference,
    owner: Subject,
    now: number,
): Promise<string> {
    // define the owner role in the scope
    const role = schema.identifier("role").parse(`role-${v7()}`);
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
                id: schema.identifier("relationship").parse(`relationship-${v7()}`),
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

/** Bind a scope's owner role to an owner once, as the scope's creation binds it, defining the role when the scope has none, and return the role. */
async function keepOwner(
    database: DatabaseConnection,
    scope: ObjectReference,
    owner: Subject,
    now: number,
): Promise<string> {
    // define the owner role with its binding when the scope has none
    const [defined] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(and(eq(accessRole.scope, scope.id), eq(accessRole.name, OWNER_ROLE.name)));
    if (defined === undefined) {
        return own(database, scope, owner, now);
    }

    // bind the defined role to the owner unless bound already
    await bindOnce(database, scope, defined.id, owner, now);

    return defined.id;
}

/** Keep a role a scope's creation defines with exactly some permissions, bound to one subject on the scope, as the owner role is, and return the role. */
async function keepInherent(
    database: DatabaseConnection,
    scope: ObjectReference,
    request: RoleRequest,
    subject: Subject,
    now: number,
): Promise<string> {
    // define the role, or replace the permissions of the one defined
    const [defined] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(and(eq(accessRole.scope, scope.id), eq(accessRole.name, request.name)));
    const role = defined?.id ?? (await define(database, scope.id, request, now, null))?.id;
    if (role === undefined) {
        throw new AccessError("CONFLICT", "role name is already in use");
    }
    if (defined !== undefined) {
        await replace(database, role, scope.id, request.permissions);
    }

    // bind it to the subject unless bound already
    await bindOnce(database, scope, role, subject, now);

    return role;
}

/** Bind a role on a scope's object to a subject unless bound already. */
async function bindOnce(
    database: DatabaseConnection,
    scope: ObjectReference,
    role: Select<typeof accessRole>["id"],
    subject: Subject,
    now: number,
): Promise<void> {
    // read the subject's binding of the role
    const bound = await database
        .select({ id: accessRelationship.id })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.roleId, role),
                eq(accessRelationship.subjectPackageId, subject.packageId),
                eq(accessRelationship.subjectType, subject.type),
                eq(accessRelationship.subjectScope, subject.scope),
                eq(accessRelationship.subjectId, subject.id),
            ),
        );
    if (bound.length > 0) {
        return;
    }

    // bind it on the scope's object
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                object: scope,
                role,
                subject,
                createdAt: now,
                expiresAt: null,
            },
            scope.id,
        ),
    );
}

/** Read a permission reference from a role permission row. */
function referenceOf(row: Select<typeof accessRolePermission>): PermissionReference {
    return { packageId: row.packageId, type: row.type, name: row.name };
}
