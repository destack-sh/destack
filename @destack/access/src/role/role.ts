import type { ObjectReference, Subject } from "@destack/sync";
import { TABLE, type RowImage, type Select, type Snapshot, type Transaction } from "@destack/db";
import { defineSchema, schema, type Identifier } from "@destack/schema";
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
    find,
    permissions,
    describe,
    close,
    keepOwner,
    keepInherent,
};

/** Read one role of a scope as a transaction sees it. */
async function read(
    transaction: Transaction,
    scope: string,
    id: string,
): Promise<Select<typeof accessRole>> {
    const record = await transaction.row(accessRole, { id: schema.identifier("role").parse(id) });
    if (record === null || record.scope !== scope) {
        throw new AccessError("NOT_FOUND", "role not found");
    }

    return record;
}

/** Find a scope's role by its name as a snapshot shows it, null for none. */
async function find(
    snapshot: Snapshot,
    scope: string,
    name: string,
): Promise<RowImage<typeof accessRole> | null> {
    const [found = null] = await snapshot.rows(accessRole, { scope, name });

    return found;
}

/** Read the permissions a role grants itself as a transaction sees them. */
async function permissions(
    transaction: Transaction,
    roleId: Select<typeof accessRole>["id"],
): Promise<PermissionReference[]> {
    const rows = await transaction.rows(
        accessRolePermission,
        { roleId },
        { orderBy: { packageId: "asc", type: "asc", name: "asc" } },
    );

    return rows.map(referenceOf);
}

/** Define a role with permissions and manager under a name, absent if the name is taken. */
export async function define(
    transaction: Transaction,
    scope: string,
    request: RoleRequest,
    now: number,
    manager: Manager | null,
): Promise<Identifier<"role"> | undefined> {
    // refuse a name another role of the scope holds
    const holding = await transaction.rows(accessRole, { scope, name: request.name }, { limit: 1 });
    if (holding.length > 0) {
        return undefined;
    }

    // write the role and its permissions
    const id = schema.identifier("role").parse(`role-${v7()}`);
    await transaction.write({
        table: accessRole,
        before: null,
        after: accessRole[TABLE].complete({
            id,
            createdAt: now,
            updatedAt: now,
            scope,
            name: request.name,
            description: request.description,
            ...Manager.values(manager),
        }),
    });
    await permit(transaction, id, scope, request.permissions);

    return id;
}

/** Write the permissions a role grants. */
export async function permit(
    transaction: Transaction,
    roleId: Select<typeof accessRole>["id"],
    scope: string,
    granted: readonly PermissionReference[],
): Promise<void> {
    await transaction.write(
        ...granted.map((permission) => ({
            table: accessRolePermission,
            before: null,
            after: accessRolePermission[TABLE].complete({
                id: schema.identifier("role-permission").parse(`role-permission-${v7()}`),
                roleId,
                scope,
                packageId: permission.packageId,
                type: permission.type,
                name: permission.name,
            }),
        })),
    );
}

/** Replace a role's permissions with a declared set, keeping rows of existing ones. */
export async function replace(
    transaction: Transaction,
    roleId: Select<typeof accessRole>["id"],
    scope: string,
    granted: readonly PermissionReference[],
): Promise<void> {
    // read the permissions the role grants
    const existing = await transaction.rows(accessRolePermission, { roleId });
    const declared = new Set(granted.map((permission) => PermissionReference.key(permission)));
    const current = new Set(existing.map((permission) => PermissionReference.key(permission)));

    // remove the ones no longer declared and add the new ones
    await transaction.write(
        ...existing
            .filter((permission) => !declared.has(PermissionReference.key(permission)))
            .map((permission) => ({
                table: accessRolePermission,
                before: permission,
                after: null,
            })),
    );
    const added = granted.filter((permission) => !current.has(PermissionReference.key(permission)));
    await permit(transaction, roleId, scope, added);
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

/** Define a scope's owner role and bind it to an owner on the scope object, returning it. */
export async function own(
    transaction: Transaction,
    scope: ObjectReference,
    owner: Subject,
    now: number,
): Promise<Identifier<"role">> {
    // define the owner role in the scope
    const role = schema.identifier("role").parse(`role-${v7()}`);
    await transaction.write({
        table: accessRole,
        before: null,
        after: accessRole[TABLE].complete({
            id: role,
            createdAt: now,
            updatedAt: now,
            scope: scope.id,
            ...OWNER_ROLE,
            isUniversal: true,
        }),
    });

    // bind it on the scope's object
    await bind(transaction, scope, role, owner, now);

    return role;
}

/** Bind a scope's owner role to an owner once, as creation does, and return the role. */
async function keepOwner(
    transaction: Transaction,
    scope: ObjectReference,
    owner: Subject,
    now: number,
): Promise<string> {
    // define the owner role with its binding when the scope has none
    const [defined] = await transaction.rows(accessRole, {
        scope: scope.id,
        name: OWNER_ROLE.name,
    });
    if (defined === undefined) {
        return own(transaction, scope, owner, now);
    }

    // bind the defined role to the owner unless bound already
    await bindOnce(transaction, scope, defined.id, owner, now);

    return defined.id;
}

/** Keep a creation-defined role with exactly some permissions, bound to one scope subject. */
async function keepInherent(
    transaction: Transaction,
    scope: ObjectReference,
    request: RoleRequest,
    subject: Subject,
    now: number,
): Promise<string> {
    // define the role, or replace the permissions of the one defined
    const [defined] = await transaction.rows(accessRole, { scope: scope.id, name: request.name });
    const role = defined?.id ?? (await define(transaction, scope.id, request, now, null));
    if (role === undefined) {
        throw new AccessError("CONFLICT", "role name is already in use");
    }
    if (defined !== undefined) {
        await replace(transaction, role, scope.id, request.permissions);
    }

    // bind it to the subject unless bound already
    await bindOnce(transaction, scope, role, subject, now);

    return role;
}

/** Bind a role on a scope's object to a subject unless bound already. */
async function bindOnce(
    transaction: Transaction,
    scope: ObjectReference,
    role: Select<typeof accessRole>["id"],
    subject: Subject,
    now: number,
): Promise<void> {
    // read the subject's binding of the role
    const bound = await transaction.rows(accessRelationship, {
        roleId: role,
        subjectPackageId: subject.packageId,
        subjectType: subject.type,
        subjectScope: subject.scope,
        subjectId: subject.id,
    });

    // bind it on the scope's object when unbound
    if (bound.length === 0) {
        await bind(transaction, scope, role, subject, now);
    }
}

/** Bind a role on a scope's object to a subject. */
async function bind(
    transaction: Transaction,
    scope: ObjectReference,
    role: Select<typeof accessRole>["id"],
    subject: Subject,
    now: number,
): Promise<void> {
    const relationship = Relationship.encode(
        {
            id: schema.identifier("relationship").parse(`relationship-${v7()}`),
            object: scope,
            role,
            subject,
            createdAt: now,
            expiresAt: null,
        },
        scope.id,
    );
    await transaction.write({
        table: accessRelationship,
        before: null,
        after: accessRelationship[TABLE].complete(relationship),
    });
}

/** Read a permission reference from a role permission row. */
function referenceOf(row: Select<typeof accessRolePermission>): PermissionReference {
    return { packageId: row.packageId, type: row.type, name: row.name };
}
