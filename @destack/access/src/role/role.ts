import type { ObjectReference, Subject } from "@destack/sync";
import { type Select, type Transaction, Table } from "@destack/db";
import { defineSchema, schema, type Identifier } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import { PermissionReference } from "../declare/policy.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission, OWNER_ROLE } from "./table.ts";

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
        /** The permissions the role grants itself. */
        permissions: schema.array(PermissionReference),
        /** The revision that conditional changes compare against. */
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

    /** The owner role's identifier, the same in every scope and database. */
    OWNER: OWNER_ROLE.id,

    /** Read one role of a scope as a transaction sees it. */
    async read(
        transaction: Transaction,
        scope: string,
        id: string,
    ): Promise<Select<typeof accessRole>> {
        const record = await transaction.read(accessRole).findFirst({
            where: {
                id: schema.identifier("role").parse(id),
            },
        });
        if (record === undefined || record.scope !== scope) {
            throw new AccessError("NOT_FOUND", "role not found");
        }

        return record;
    },

    /** Read the permissions a role grants itself as a transaction sees them. */
    async permissions(
        transaction: Transaction,
        roleId: Select<typeof accessRole>["id"],
    ): Promise<PermissionReference[]> {
        const rows = await transaction.read(accessRolePermission).findMany({
            where: { roleId },
            orderBy: { packageId: "asc", type: "asc", name: "asc" },
        });

        return rows.map((row) => Role.reference(row));
    },

    /** Define a role with permissions and manager under a name, absent if the name is taken. */
    async define(
        transaction: Transaction,
        scope: string,
        request: RoleRequest,
        now: number,
        manager: Manager | null,
    ): Promise<Identifier<"role"> | undefined> {
        // refuse a name another role of the scope has
        const holding = await transaction
            .read(accessRole)
            .findMany({ where: { scope, name: request.name }, limit: 1 });
        if (holding.length > 0) {
            return undefined;
        }

        // write the role and its permissions
        const id = schema.identifier("role").parse(`role-${v7()}`);
        await transaction.write({
            table: accessRole,
            before: null,
            after: Table.complete(accessRole, {
                id,
                createdAt: now,
                updatedAt: now,
                scope,
                name: request.name,
                description: request.description,
                ...Manager.values(manager),
            }),
        });
        await Role.permit(transaction, id, scope, request.permissions);

        return id;
    },

    /** Write the permissions a role grants. */
    async permit(
        transaction: Transaction,
        roleId: Select<typeof accessRole>["id"],
        scope: string,
        granted: readonly PermissionReference[],
    ): Promise<void> {
        await transaction.write(
            ...granted.map((permission) => ({
                table: accessRolePermission,
                before: null,
                after: Table.complete(accessRolePermission, {
                    id: schema.identifier("role-permission").parse(`role-permission-${v7()}`),
                    roleId,
                    scope,
                    packageId: permission.packageId,
                    type: permission.type,
                    name: permission.name,
                }),
            })),
        );
    },

    /** Replace a role's permissions with a declared set, keeping rows of existing ones. */
    async replace(
        transaction: Transaction,
        roleId: Select<typeof accessRole>["id"],
        scope: string,
        granted: readonly PermissionReference[],
    ): Promise<void> {
        // read the permissions the role grants
        const existing = await transaction
            .read(accessRolePermission)
            .findMany({ where: { roleId } });
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
        const added = granted.filter(
            (permission) => !current.has(PermissionReference.key(permission)),
        );
        await Role.permit(transaction, roleId, scope, added);
    },

    /** Describe a role row and its permissions. */
    describe(record: Select<typeof accessRole>, granted: readonly PermissionReference[]): Role {
        return {
            id: record.id,
            scope: record.scope,
            name: record.name,
            description: record.description,
            permissions: [...granted],
            revision: record.revision,
        };
    },

    /** Bind the owner role on a scope's object to an owner once. */
    async bindOwner(
        transaction: Transaction,
        scope: ObjectReference,
        owner: Subject,
        now: number,
    ): Promise<void> {
        await Role.bindOnce(transaction, scope, Role.OWNER, owner, now);
    },

    /** Keep a creation-defined role with exactly some permissions, bound to one scope subject. */
    async keepInherent(
        transaction: Transaction,
        scope: ObjectReference,
        request: RoleRequest,
        subject: Subject,
        now: number,
    ): Promise<string> {
        // define the role, or replace the permissions of the one defined
        const defined = await transaction.read(accessRole).findFirst({
            where: {
                scope: scope.id,
                name: request.name,
            },
        });
        const role = defined?.id ?? (await Role.define(transaction, scope.id, request, now, null));
        if (role === undefined) {
            throw new AccessError("CONFLICT", "role name is already in use");
        }
        if (defined !== undefined) {
            await Role.replace(transaction, role, scope.id, request.permissions);
        }

        // bind it to the subject unless bound already
        await Role.bindOnce(transaction, scope, role, subject, now);

        return role;
    },

    /** Bind a role on a scope's object to a subject unless bound already. */
    async bindOnce(
        transaction: Transaction,
        scope: ObjectReference,
        role: Select<typeof accessRole>["id"],
        subject: Subject,
        now: number,
    ): Promise<void> {
        // read the subject's binding of the role on the scope's object
        const bound = await transaction.read(accessRelationship).findMany({
            where: {
                ...Relationship.objectColumns(scope),
                roleId: role,
                subjectPackageId: subject.packageId,
                subjectType: subject.type,
                subjectScope: subject.scope,
                subjectId: subject.id,
            },
        });

        // bind it on the scope's object when unbound
        if (bound.length === 0) {
            await Role.bind(transaction, scope, role, subject, now);
        }
    },

    /** Bind a role on a scope's object to a subject. */
    async bind(
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
            after: Table.complete(accessRelationship, relationship),
        });
    },

    /** Read a permission reference from a role permission row. */
    reference(row: Select<typeof accessRolePermission>): PermissionReference {
        return { packageId: row.packageId, type: row.type, name: row.name };
    },
};
