import { Scope } from "@destack/sync";
import { readGrant, relationship, role } from "@destack/object";
import { present } from "@destack/schema";
import {
    accessRolePermission,
    Manager,
    PermissionReference,
    Relationship,
    Role,
    type RelationshipRequest,
} from "@destack/access";

/** Implement the roles a stack declares in its space. */
export function implementRole() {
    return declaredRole;
}

/** Implement the relationships a stack declares in its space. */
export function implementRelationship() {
    return declaredRelationship;
}

/** Roles a stack declares in its space, after every object their permissions may name. */
const declaredRole = role.implement({
    declaration: {
        after: "every",
        values: (name, resolved) => ({
            name,
            description: resolved.description,
        }),
        changed: async (transaction, row, resolved) => {
            const current = await transaction
                .read(accessRolePermission)
                .findMany({ where: { roleId: row.id } });

            return (
                JSON.stringify(sortPermissions(current)) !==
                JSON.stringify(sortPermissions(resolved.permissions))
            );
        },
        upsert: async (stack, manager, resolved) => {
            // keep the role with its manager and permissions as the stack's principal in one write
            const scope = await Scope.object(stack.transaction.view(), stack.scope);
            const authorization = (await stack.authorization()).applying(manager);
            await Role.upsert(
                scope,
                {
                    name: manager.name,
                    description: resolved.description,
                    permissions: resolved.permissions,
                },
                authorization,
            );
        },
        retire: {
            isRetiring: async () => false,
            start: async (stack, row) => {
                // delete the role the stack no longer declares, through its declaration
                const scope = await Scope.object(stack.transaction.view(), row.scope);
                const manager = present(Manager.read(row), `the declaration managing ${row.id}`);
                const authorization = (await stack.authorization()).applying(manager);
                await Role.delete(scope, row.id, row.revision, authorization);
            },
        },
    },
});

/** Relationships a stack declares in its space, their references resolved by the stack. */
const declaredRelationship = relationship.implement({
    declaration: {
        resolve: async (_name, input, stack) => {
            // require a relationship through exactly one relation or role that access accepts
            const { object, ...granted } = input;
            const request: RelationshipRequest = { object, ...readGrant(granted) };
            stack.authorizer.validate(
                { ...request, expiresAt: request.expiresAt ?? null },
                stack.now,
            );

            return request;
        },
        values: (_name, resolved, stack) => ({
            scope: stack.scope,
            ...Relationship.objectColumns(resolved.object),
            ...Relationship.viaColumns(resolved),
            ...Relationship.subjectColumns(resolved.subject),
            expiresAt: resolved.expiresAt ?? null,
        }),
        upsert: async (stack, manager, resolved) => {
            // keep the one relationship the declaration manages as the stack's principal and replace a changed one
            const authorization = (await stack.authorization()).applying(manager);
            await Relationship.replace({ scope: stack.scope }, [resolved], authorization);
        },
        retire: {
            isRetiring: async () => false,
            start: async (stack, row) => {
                // revoke the relationship the stack no longer declares, through its declaration
                const manager = present(Manager.read(row), `the declaration managing ${row.id}`);
                const authorization = (await stack.authorization()).applying(manager);
                await Relationship.revoke(Relationship.decode(row).object, row.id, authorization);
            },
        },
    },
});

/** Order permissions for comparison independently of declaration order. */
function sortPermissions(
    permissions: readonly Pick<PermissionReference, "packageId" | "type" | "name">[],
) {
    return permissions.map((permission) => PermissionReference.key(permission)).toSorted();
}
