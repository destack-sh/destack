import * as policies from "../declare/principal.ts";
import type { Authorization } from "../authorizer/authorization.ts";
import { type ObjectReference, Subject } from "@destack/sync";
import { type Select, type Transaction, Table } from "@destack/db";
import { Identifier, type UuidEnvironment, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import { PermissionReference } from "../declare/policy.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission, OWNER_ROLE } from "./table.ts";

/** The schema of a role: a named set of permissions a scope defines. */
const roleSchema = schema.object({
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
});
/** A named set of permissions a scope defines and relationships bind to subjects. */
export type Role = schema.Infer<typeof roleSchema>;

/** The name, purpose and permissions of a role to define or change. */
export const RoleRequest = schema.object({
    /** The name, unique within the scope. */
    name: schema.string().min(1).max(200),
    /** The purpose shown when granting the role. */
    description: schema.string().max(1000),
    /** The permissions the role grants. */
    permissions: schema.array(PermissionReference),
});
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
            throw AccessError.of("ROLE_NOT_FOUND", "role not found");
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
        environment: UuidEnvironment,
    ): Promise<Identifier<"role"> | undefined> {
        // refuse a name another role of the scope has
        const holding = await transaction
            .read(accessRole)
            .findMany({ where: { scope, name: request.name }, limit: 1 });
        if (holding.length > 0) {
            return undefined;
        }

        // write the role and its permissions
        const id = Identifier.create("role", environment);
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
        await Role.permit(transaction, id, scope, request.permissions, environment);

        return id;
    },

    /** Write the permissions a role grants. */
    async permit(
        transaction: Transaction,
        roleId: Select<typeof accessRole>["id"],
        scope: string,
        granted: readonly PermissionReference[],
        environment: UuidEnvironment,
    ): Promise<void> {
        await transaction.write(
            ...granted.map((permission) => ({
                table: accessRolePermission,
                before: null,
                after: Table.complete(accessRolePermission, {
                    id: Identifier.create("role-permission", environment),
                    roleId,
                    scope,
                    packageId: permission.packageId,
                    type: permission.type,
                    name: permission.name,
                }),
            })),
        );
    },

    /** Replace a role's permissions with a declared set, leaving the rows of unchanged ones. */
    async replace(
        transaction: Transaction,
        roleId: Select<typeof accessRole>["id"],
        scope: string,
        granted: readonly PermissionReference[],
        environment: UuidEnvironment,
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
        await Role.permit(transaction, roleId, scope, added, environment);
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

    /** Apply a creation-defined role: exact permissions bound to a subject, dropped for none. */
    async applyInherent(
        scope: ObjectReference,
        request: RoleRequest,
        subject: Subject,
        authorization: Authorization,
    ): Promise<void> {
        // refuse a scope another database homes
        const { transaction, environment } = authorization;
        await authorization.authorizer.requireHome(transaction, scope, scope.id);

        // drop the role and its bindings once its code owns nothing
        const defined = await transaction.read(accessRole).findFirst({
            where: { scope: scope.id, name: request.name },
        });
        if (request.permissions.length === 0) {
            if (defined !== undefined) {
                await Role.drop(defined, authorization);
            }

            return;
        }

        // define the role, or replace the permissions of the one defined
        const now = authorization.context().now;
        const role =
            defined?.id ??
            (await Role.define(transaction, scope.id, request, now, null, environment));
        if (role === undefined) {
            throw AccessError.of("ROLE_NAME_TAKEN", "role name is already in use");
        }
        if (defined !== undefined) {
            await Role.replace(transaction, role, scope.id, request.permissions, environment);
        }

        // bind it to the subject
        await Role.bind(scope, role, subject, authorization);
    },

    /** Drop a code-owned role with its bindings and permissions. */
    async drop(record: Select<typeof accessRole>, authorization: Authorization): Promise<void> {
        // remove its bindings
        const { transaction, environment } = authorization;
        const bindings = await transaction
            .read(accessRelationship)
            .findMany({ where: { roleId: record.id } });
        for (const binding of bindings) {
            await transaction.write({ table: accessRelationship, before: binding, after: null });
        }

        // remove its permissions and the role
        await Role.replace(transaction, record.id, record.scope, [], environment);
        await transaction.write({ table: accessRole, before: record, after: null });
    },

    /** Bind a role on a scope's object to a subject once, at the home of the scope keeping it. */
    async bind(
        scope: ObjectReference,
        role: Select<typeof accessRole>["id"],
        subject: Subject,
        authorization: Authorization,
    ): Promise<void> {
        // refuse a binding whose scope another database homes
        const { transaction, snapshot } = authorization;
        const placed = await Relationship.scope(snapshot, { object: scope, role });
        await authorization.authorizer.requireHome(transaction, scope, placed);

        // leave a binding the subject has
        const bound = await transaction.read(accessRelationship).findMany({
            where: {
                ...Relationship.objectColumns(scope),
                roleId: role,
                subjectPackageId: subject.packageId,
                subjectType: subject.type,
                subjectScope: subject.scope,
                subjectId: subject.id,
            },
            limit: 1,
        });
        if (bound.length > 0) {
            return;
        }

        // write the binding in its scope
        const relationship = Relationship.encode(
            {
                id: Identifier.create("relationship", authorization.environment),
                object: scope,
                role,
                subject,
                createdAt: authorization.context().now,
                expiresAt: null,
            },
            placed,
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

    /** Define a role under a free name, granting only permissions the caller has there. */
    async create(
        scope: ObjectReference,
        request: RoleRequest,
        authorization: Authorization,
    ): Promise<Role> {
        return authorization.transaction.savepoint(async () => {
            // require permission to define roles and every permission the role grants
            await authorization.authorizeRole(scope, [
                policies.role.permission("create"),
                ...request.permissions,
            ]);

            // define the role under a name free in the scope
            const id = await Role.define(
                authorization.transaction,
                scope.id,
                request,
                authorization.context().now,
                authorization.manager,
                authorization.environment,
            );
            if (id === undefined) {
                throw AccessError.of("ROLE_NAME_TAKEN", "role name is already in use");
            }
            const record = await Role.read(authorization.transaction, scope.id, id);

            return Role.describe(record, request.permissions);
        });
    },

    /** Change a role's name, purpose or permissions at a revision, within the caller's rights. */
    async update(
        scope: ObjectReference,
        id: string,
        changes: Partial<RoleRequest> & { readonly revision: number },
        authorization: Authorization,
    ): Promise<Role> {
        return authorization.transaction.savepoint(async () => {
            // require an editable role at the revision, permission to change roles, and every permission granted
            const context = authorization.context();
            const record = await Role.read(authorization.transaction, scope.id, id);
            if (record.revision !== changes.revision) {
                throw AccessError.of("REVISION_STALE", "role revision has changed");
            }
            await authorization.authorizeRole(
                scope,
                [policies.role.permission("update"), ...(changes.permissions ?? [])],
                record,
            );

            // apply the changes at the next revision
            const updated = {
                ...record,
                ...(changes.name === undefined ? {} : { name: changes.name }),
                ...(changes.description === undefined ? {} : { description: changes.description }),
                revision: record.revision + 1,
                updatedAt: context.now,
            };
            await authorization.transaction.write({
                table: accessRole,
                before: record,
                after: updated,
            });

            // replace the permissions when given
            if (changes.permissions !== undefined) {
                await Role.replace(
                    authorization.transaction,
                    record.id,
                    scope.id,
                    changes.permissions,
                    authorization.environment,
                );
            }

            return Role.describe(
                updated,
                changes.permissions ??
                    (await Role.permissions(authorization.transaction, record.id)),
            );
        });
    },

    /** Delete an unbound role at a revision, managed roles only through their declaration. */
    async delete(
        scope: ObjectReference,
        id: string,
        revision: number,
        authorization: Authorization,
    ): Promise<Role> {
        return authorization.transaction.savepoint(async () => {
            // require an editable role at the revision and permission to delete roles
            const record = await Role.read(authorization.transaction, scope.id, id);
            if (record.revision !== revision) {
                throw AccessError.of("REVISION_STALE", "role revision has changed");
            }
            await authorization.authorizeRole(scope, [policies.role.permission("delete")], record);

            // refuse deleting a role that is still bound or included
            const bindings = await authorization.transaction.read(accessRelationship).findMany({
                where: {
                    OR: [
                        { roleId: record.id },
                        {
                            subjectPackageId: policies.role.definition.packageId,
                            subjectType: policies.role.name,
                            subjectId: record.id,
                        },
                    ],
                },
                limit: 1,
            });
            if (bindings.length > 0) {
                throw AccessError.of("ROLE_IN_USE", "role is still bound or included");
            }

            // delete the role with its permissions
            const permissions = await Role.permissions(authorization.transaction, record.id);
            await Role.replace(
                authorization.transaction,
                record.id,
                scope.id,
                [],
                authorization.environment,
            );
            await authorization.transaction.write({
                table: accessRole,
                before: record,
                after: null,
            });

            return Role.describe(record, permissions);
        });
    },

    /** Upsert a role by name in a scope with exactly the requested purpose and permissions. */
    async upsert(
        scope: ObjectReference,
        request: RoleRequest,
        authorization: Authorization,
    ): Promise<Role> {
        return authorization.transaction.savepoint(async () => {
            // read the role with the name, and define it when none exists
            const now = authorization.context().now;
            const existing = await authorization.transaction.read(accessRole).findFirst({
                where: {
                    scope: scope.id,
                    name: request.name,
                },
            });
            if (existing === undefined) {
                return Role.create(scope, request, authorization);
            }

            // accept only a role under the same manager, and leave a matching role as it is
            await authorization.authorizeRole(
                scope,
                [policies.role.permission("update"), ...request.permissions],
                existing,
            );

            // refuse a role of the name the declaration does not manage
            const { manager } = authorization;
            if (manager !== null && !Manager.manages(existing, manager)) {
                throw AccessError.of("ROLE_NAME_TAKEN", "role name is already in use");
            }
            const granted = await Role.permissions(authorization.transaction, existing.id);
            const isGranted = isSamePermissions(granted, request.permissions);
            if (existing.description === request.description && isGranted) {
                return Role.describe(existing, granted);
            }

            // describe it anew at its next revision, replacing differing permissions
            const updated = {
                ...existing,
                description: request.description,
                updatedAt: now,
                revision: existing.revision + 1,
            };
            await authorization.transaction.write({
                table: accessRole,
                before: existing,
                after: updated,
            });
            if (!isGranted) {
                await Role.replace(
                    authorization.transaction,
                    existing.id,
                    scope.id,
                    request.permissions,
                    authorization.environment,
                );
            }

            return Role.describe(updated, request.permissions);
        });
    },

    /** Bind a role on an object to exactly some subjects, revoking its other bindings. */
    async assign(
        object: ObjectReference,
        role: string,
        subjects: readonly Subject[],
        authorization: Authorization,
    ): Promise<Relationship[]> {
        return authorization.transaction.savepoint(async () => {
            // read the role's bindings, and split those of the subjects on the object from the rest
            const id = schema.identifier("role").parse(role);
            const rows = await authorization.transaction
                .read(accessRelationship)
                .findMany({ where: { roleId: id } });
            const isWanted = (binding: Relationship) =>
                subjects.some((subject) => isBinding(binding, { object, role, subject }));
            const bound = rows.map((row) => ({ row, binding: Relationship.decode(row) }));
            const stale = bound.filter(({ binding }) => !isWanted(binding));

            // revoke the other bindings, as the caller may
            for (const { row, binding } of stale) {
                await authorization.authorizeRevoke(binding, row);
                await authorization.transaction.write({
                    table: accessRelationship,
                    before: row,
                    after: null,
                });
            }

            // bind each subject once
            const assigned: Relationship[] = [];
            for (const subject of subjects) {
                const request = { object, role, subject };
                const kept = bound.find(({ binding }) => isBinding(binding, request))?.binding;

                // keep an existing binding as it is
                if (kept !== undefined) {
                    assigned.push(kept);
                }
                // bind a missing one, as the caller may
                else {
                    const scope = await authorization.authorizeGrant(request);
                    assigned.push(await Relationship.create(request, scope, authorization));
                }
            }

            return assigned;
        });
    },

    /** Upsert a scope's role by name with exactly some permissions, bound to one subject. */
    async upsertBinding(
        scope: ObjectReference,
        binding: Omit<RoleRequest, "permissions"> & {
            /** The permissions the role grants, none to drop the role and its binding. */
            readonly permissions: RoleRequest["permissions"];
            /** The one subject the role is bound to. */
            readonly subject: Subject;
            /** The object in the scope the role is bound on, the scope itself when absent. */
            readonly object?: ObjectReference;
        },
        authorization: Authorization,
    ): Promise<void> {
        // drop the binding and the role when it grants nothing
        const { subject, object, ...request } = binding;
        if (request.permissions.length === 0) {
            await Relationship.replace({ scope: scope.id }, [], authorization);
            const existing = await authorization.transaction.read(accessRole).findFirst({
                where: {
                    scope: scope.id,
                    name: request.name,
                },
            });
            if (existing !== undefined) {
                await Role.delete(scope, existing.id, existing.revision, authorization);
            }

            return;
        }

        // upsert the role with exactly the permissions, bound only to the subject
        const role = await Role.upsert(scope, request, authorization);
        await Relationship.replace(
            { scope: scope.id },
            [
                {
                    object: object ?? scope,
                    role: role.id,
                    subject,
                },
            ],
            authorization,
        );
    },
};

/** Report whether a relationship binds a role to a subject without condition or expiry. */
function isBinding(
    relationship: Relationship,
    request: {
        readonly object: ObjectReference;
        readonly role: string;
        readonly subject: Subject;
    },
): boolean {
    return (
        "role" in relationship &&
        relationship.role === request.role &&
        Subject.same(relationship.object, request.object) &&
        Subject.same(relationship.subject, request.subject) &&
        relationship.expiresAt === null &&
        relationship.linkSecret === undefined
    );
}

/** Report whether two lists of permissions grant the same set. */
function isSamePermissions(
    left: readonly PermissionReference[],
    right: readonly PermissionReference[],
): boolean {
    // compare the sets of their keys
    const lefts = new Set(left.map((permission) => PermissionReference.key(permission)));
    const rights = new Set(right.map((permission) => PermissionReference.key(permission)));

    return lefts.size === rights.size && [...lefts].every((key) => rights.has(key));
}
