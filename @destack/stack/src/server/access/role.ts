import { Scope, type Subject } from "@destack/sync";
import type { ObjectType } from "@destack/object";
import type { Stack } from "@destack/object/server";
import { present, schema, type Identifier } from "@destack/schema";
import {
    accessRolePermission,
    anyone,
    Manager,
    principal,
    PermissionReference,
    Relationship,
    type RelationshipRequest,
} from "@destack/access";
import { group } from "@destack/account/object";
import type {
    StackObjectReference,
    RelationshipDefinition,
    StackRoleReference,
} from "../../declare/permission.ts";
import { StackError } from "../../error/index.ts";
import { installation, relationship, role } from "../../object/index.ts";

/** Serve the roles and relationships a stack declares in its space. */
export function serveRoles() {
    return [declaredRole, declaredRelationship] as const;
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
        keep: async (stack, manager, resolved) => {
            // keep the role with its manager and permissions as the stack's principal in one write
            const scope = await Scope.object(stack.transaction.view(), stack.scope);
            await (await stack.authorization()).applying(manager).keepRole(scope, {
                name: manager.name,
                description: resolved.description,
                permissions: resolved.permissions,
            });
        },
        retire: {
            isRetiring: async () => false,
            start: async (stack, row) => {
                // delete the role the stack no longer declares, through its declaration
                const scope = await Scope.object(stack.transaction.view(), row.scope);
                const manager = present(Manager.read(row), `the declaration managing ${row.id}`);
                await (
                    await stack.authorization()
                )
                    .applying(manager)
                    .deleteRole(scope, row.id, row.revision);
            },
        },
    },
});

/** Relationships a stack declares in its space, with objects, roles and subjects resolved. */
const declaredRelationship = relationship.implement({
    declaration: {
        after: "every",
        resolve: async (_name, declared, stack) => {
            // relate the declared object or the space itself
            const spaceId = schema.identifier("space").parse(stack.scope);
            const object =
                declared.object === undefined
                    ? await Scope.object(stack.transaction.view(), spaceId)
                    : await objectReference(declared.object, stack);

            // require a relationship access accepts
            const via =
                "relation" in declared
                    ? { relation: declared.relation }
                    : { role: await roleIdentifier(declared.role, stack) };
            const subject = await subjectReference(declared.subject, spaceId, stack);
            stack.authorizer.validate(
                { object, ...via, subject, expiresAt: declared.expiresAt ?? null },
                stack.now,
            );
            const request: RelationshipRequest = {
                object,
                ...via,
                subject,
                ...(declared.expiresAt === undefined ? {} : { expiresAt: declared.expiresAt }),
            };

            return request;
        },
        values: (_name, resolved, stack) => ({
            scope: stack.scope,
            ...Relationship.objectColumns(resolved.object),
            ...Relationship.viaColumns(resolved),
            ...Relationship.subjectColumns(resolved.subject),
            expiresAt: resolved.expiresAt ?? null,
        }),
        keep: async (stack, manager, resolved) => {
            // keep the one relationship the declaration manages as the stack's principal and replace a changed one
            await (
                await stack.authorization()
            )
                .applying(manager)
                .keepRelationships({ scope: stack.scope }, [resolved]);
        },
        retire: {
            isRetiring: async () => false,
            start: async (stack, row) => {
                // revoke the relationship the stack no longer declares, through its declaration
                const manager = present(Manager.read(row), `the declaration managing ${row.id}`);
                await (
                    await stack.authorization()
                )
                    .applying(manager)
                    .revoke(Relationship.decode(row).object, row.id);
            },
        },
    },
});

/** Reference an object the stack declares or selects by identifier. */
async function objectReference(reference: schema.Infer<typeof StackObjectReference>, stack: Stack) {
    // find the type by the declaring collection's object or by the identifier's prefix
    const type = "id" in reference ? stack.findType(reference.id) : stack.object(reference.kind);
    const id =
        "id" in reference
            ? await requireIn(type, reference.id, stack)
            : await stack.require(type, reference.name);

    return type.reference(stack.scope, id);
}

/** Resolve a role the stack declares or selects by identifier. */
async function roleIdentifier(
    reference: schema.Infer<typeof StackRoleReference>,
    stack: Stack,
): Promise<Identifier<"role">> {
    return schema
        .identifier("role")
        .parse(
            typeof reference === "string"
                ? await stack.require(role, reference)
                : await requireIn(role, reference.id, stack),
        );
}

/** Resolve a declared subject to the principal, subject set or role it refers to. */
async function subjectReference(
    subject: schema.Infer<typeof RelationshipDefinition>["subject"],
    spaceId: Identifier<"space">,
    stack: Stack,
): Promise<Subject> {
    // relate a user
    if ("user" in subject) {
        return principal.user.reference(Scope.universe.id, subject.user);
    }
    // relate an installation of this space
    else if ("installation" in subject) {
        const id =
            typeof subject.installation === "string"
                ? await stack.require(installation, subject.installation)
                : await requireIn(installation, subject.installation.id, stack);

        return principal.installation.reference(spaceId, id);
    }
    // relate a role of this space
    else if ("role" in subject) {
        return role.reference(spaceId, await roleIdentifier(subject.role, stack));
    }
    // relate every caller through the wildcard
    else if ("anyone" in subject) {
        return anyone.reference("*", "*");
    }

    // relate members of the account containing the space: a group's, or the account's own
    else {
        const accountId = (await Scope.object(stack.transaction.view(), spaceId)).scope;

        return "group" in subject
            ? { ...group.reference(accountId, subject.group), relation: "member" }
            : {
                  ...(await Scope.object(stack.transaction.view(), accountId)),
                  relation: "member",
              };
    }
}

/** Require an object of a type by identifier in the stack's space. */
async function requireIn(type: ObjectType, id: string, stack: Stack): Promise<string> {
    // require the object in the space
    const [found] = await stack.transaction
        .read(type.table)
        .findMany({ where: { id, scope: stack.scope } });
    if (found === undefined) {
        throw new StackError("INVALID_DEFINITION", `the space has no ${type.name} ${id}`);
    }

    return id;
}

/** Order permissions for comparison independently of declaration order. */
function sortPermissions(
    permissions: readonly Pick<PermissionReference, "packageId" | "type" | "name">[],
) {
    return permissions.map((permission) => PermissionReference.key(permission)).toSorted();
}
