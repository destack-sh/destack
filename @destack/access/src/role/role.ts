import * as policies from "../declare/principal.ts";
import type { Authorization } from "../authorizer/authorization.ts";
import { ObjectReference, Subject } from "@destack/sync";
import { type Select, type Transaction, Table } from "@destack/db";
import { Identifier, type UuidEnvironment, schema } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Manager } from "../manager/manager.ts";
import { PermissionReference } from "../declare/policy.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { accessRole, accessRolePermission, OWNER_ROLE } from "./table.ts";

/** The most approvals a role's permission waits for: ten, as GitHub rulesets allow. */
const MAX_APPROVALS = 10;

/** The schema of a role permission: a permission, and the approvals its uses wait for. */
const rolePermissionSchema = PermissionReference.extend({
    /** The approvals each use waits for, none to run it at once. */
    approvals: schema.number().int().min(0).max(MAX_APPROVALS).exactOptional(),
});
/** A permission a role grants, at once or once a change gathers some approvals. */
export type RolePermission = schema.Infer<typeof rolePermissionSchema>;

/** A permission a role grants, at once or once a change gathers some approvals. */
export const RolePermission = Object.assign(rolePermissionSchema, {
    /** Refuse a list granting a permission twice, at once and on approvals or at two counts. */
    requireDistinct(permissions: readonly RolePermission[]): void {
        const keys = permissions.map((each) => PermissionReference.key(each));
        const repeated = permissions.find(
            (each, index) => keys.indexOf(keys[index] ?? "") !== index,
        );
        if (repeated !== undefined) {
            throw AccessError.of(
                "PERMISSION_INVALID",
                `role permission '${repeated.type}.${repeated.name}' is invalid: a role grants it once`,
            );
        }
    },

    /** Key a role permission by its permission and the approvals its uses wait for. */
    key(permission: RolePermission): string {
        return `${PermissionReference.key(permission)}@${permission.approvals ?? 0}`;
    },
});

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
    permissions: schema.array(RolePermission),
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
    /** The permissions the role grants, each once. */
    permissions: schema.array(RolePermission),
});
/** The name, purpose and permissions of a role to define or change. */
export type RoleRequest = schema.Infer<typeof RoleRequest>;

/** A declared role: its name, purpose and permissions, and exactly its subjects. */
export const RoleDeclaration = RoleRequest.extend({
    /** The subjects the role is bound to, each once. */
    subjects: schema.array(Subject),
    /** The object in the scope the role is bound on, the scope itself when absent. */
    object: ObjectReference.exactOptional(),
});
/** A role a declaration applies, with exactly the subjects bound to it. */
export type RoleDeclaration = schema.Infer<typeof RoleDeclaration>;

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
    ): Promise<RolePermission[]> {
        const rows = await transaction.read(accessRolePermission).findMany({
            where: { roleId },
            orderBy: { packageId: "asc", type: "asc", name: "asc" },
        });

        return rows.map((row) => Role.permission(row));
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
        // refuse a permission granted twice, and a name another role of the scope has
        RolePermission.requireDistinct(request.permissions);
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
        granted: readonly RolePermission[],
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
                    approvals: permission.approvals ?? 0,
                }),
            })),
        );
    },

    /** Replace a role's permissions with a declared set, leaving the rows of unchanged ones. */
    async replace(
        transaction: Transaction,
        roleId: Select<typeof accessRole>["id"],
        scope: string,
        granted: readonly RolePermission[],
        environment: UuidEnvironment,
    ): Promise<void> {
        // refuse a permission granted twice, then read the permissions the role grants
        RolePermission.requireDistinct(granted);
        const existing = await transaction
            .read(accessRolePermission)
            .findMany({ where: { roleId } });
        const declared = new Set(granted.map((permission) => RolePermission.key(permission)));
        const current = new Set(existing.map((row) => RolePermission.key(Role.permission(row))));

        // remove the ones no longer declared and add the new ones
        await transaction.write(
            ...existing
                .filter((row) => !declared.has(RolePermission.key(Role.permission(row))))
                .map((permission) => ({
                    table: accessRolePermission,
                    before: permission,
                    after: null,
                })),
        );
        const added = granted.filter((permission) => !current.has(RolePermission.key(permission)));
        await Role.permit(transaction, roleId, scope, added, environment);
    },

    /** Describe a role row and its permissions. */
    describe(record: Select<typeof accessRole>, granted: readonly RolePermission[]): Role {
        return {
            id: record.id,
            scope: record.scope,
            name: record.name,
            description: record.description,
            permissions: [...granted],
            revision: record.revision,
        };
    },

    /** Bind a role on a scope's object to a subject once, at the home of the scope keeping it. */
    async bind(
        scope: ObjectReference,
        role: Select<typeof accessRole>["id"],
        subject: Subject,
        authorization: Authorization,
    ): Promise<void> {
        // build the binding and read whether the subject has it, together
        const { transaction } = authorization;
        const [binding, bound] = await Promise.all([
            Role.binding(scope, role, subject, authorization),
            transaction.read(accessRelationship).findMany({
                where: {
                    ...Relationship.objectColumns(scope),
                    roleId: role,
                    subjectPackageId: subject.packageId,
                    subjectType: subject.type,
                    subjectScope: subject.scope,
                    subjectId: subject.id,
                },
                limit: 1,
            }),
        ]);

        // write a binding the subject lacks
        if (bound.length === 0) {
            await transaction.write({ table: accessRelationship, before: null, after: binding });
        }
    },

    /** Build a role binding in its scope, refusing a scope another database homes. */
    async binding(
        scope: ObjectReference,
        role: Select<typeof accessRole>["id"],
        subject: Subject,
        authorization: Authorization,
    ): Promise<Select<typeof accessRelationship>> {
        // place the binding, refusing a scope another database homes
        const { transaction, snapshot } = authorization;
        const placed = await Relationship.scope(snapshot, { object: scope, role });
        await authorization.authorizer.requireHome(transaction, scope, placed);

        // encode the binding in its scope
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

        return Table.complete(accessRelationship, relationship);
    },

    /** Read a role permission from its row, its approvals only when its uses wait for some. */
    permission(row: Select<typeof accessRolePermission>): RolePermission {
        const { packageId, type, name, approvals } = row;

        return approvals === 0 ? { packageId, type, name } : { packageId, type, name, approvals };
    },

    /** Define a role under a free name, granting only permissions the caller has there. */
    async create(
        scope: ObjectReference,
        request: RoleRequest,
        authorization: Authorization,
    ): Promise<Role> {
        return authorization.transaction.savepoint(async () => {
            // require permission to define roles and every permission the role grants
            await authorization.authorizeRole(
                scope,
                policies.role.permission("create"),
                request.permissions,
            );

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
                throw AccessError.of("ROLE_NAME_EXISTS", "role name already exists");
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
                throw AccessError.of(
                    "REVISION_CONFLICT",
                    "cannot update the role: its revision changed",
                );
            }
            await authorization.authorizeRole(
                scope,
                policies.role.permission("update"),
                changes.permissions ?? [],
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
                throw AccessError.of(
                    "REVISION_CONFLICT",
                    "cannot update the role: its revision changed",
                );
            }
            await authorization.authorizeRole(
                scope,
                policies.role.permission("delete"),
                [],
                record,
            );

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
                throw AccessError.of(
                    "ROLE_BLOCKED",
                    "cannot delete the role: it is still bound or included",
                );
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

    /** Declare a role with exactly its permissions and subjects, removed once it grants none. */
    async declare(
        scope: ObjectReference,
        declaration: RoleDeclaration,
        authorization: Authorization,
    ): Promise<Role | undefined> {
        return authorization.transaction.savepoint(async () => {
            // read the role of the name
            const { subjects, object = scope, ...request } = declaration;
            const existing = await authorization.transaction.read(accessRole).findFirst({
                where: { scope: scope.id, name: request.name },
            });

            // remove a role that grants nothing, with its bindings
            if (request.permissions.length === 0) {
                if (existing !== undefined) {
                    await Role.assign(object, existing.id, [], authorization);
                    await Role.delete(scope, existing.id, existing.revision, authorization);
                }

                return undefined;
            }

            // upsert the role, then bind exactly its subjects
            const role = await upsertRole(scope, request, existing, authorization);
            await Role.assign(object, role.id, subjects, authorization);

            return role;
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
};

/** Upsert a role by name with exactly a purpose and permissions, leaving a matching one. */
async function upsertRole(
    scope: ObjectReference,
    request: RoleRequest,
    existing: Select<typeof accessRole> | undefined,
    authorization: Authorization,
): Promise<Role> {
    // define the role when none of the name exists
    if (existing === undefined) {
        return await Role.create(scope, request, authorization);
    }

    // require permission to change it, and the declaration managing it
    await authorization.authorizeRole(
        scope,
        policies.role.permission("update"),
        request.permissions,
        existing,
    );
    const { manager } = authorization;
    if (manager !== null && !Manager.manages(existing, manager)) {
        throw AccessError.of("ROLE_NAME_EXISTS", "role name already exists");
    }

    // leave a matching role as it is
    const granted = await Role.permissions(authorization.transaction, existing.id);
    const isGranted = isSamePermissions(granted, request.permissions);
    if (existing.description === request.description && isGranted) {
        return Role.describe(existing, granted);
    }

    // describe it anew at its next revision, replacing differing permissions
    const updated = {
        ...existing,
        description: request.description,
        updatedAt: authorization.context().now,
        revision: existing.revision + 1,
    };
    await authorization.transaction.write({ table: accessRole, before: existing, after: updated });
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
}

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
    left: readonly RolePermission[],
    right: readonly RolePermission[],
): boolean {
    // compare the sets of their keys
    const lefts = new Set(left.map((permission) => RolePermission.key(permission)));
    const rights = new Set(right.map((permission) => RolePermission.key(permission)));

    return lefts.size === rights.size && [...lefts].every((key) => rights.has(key));
}
