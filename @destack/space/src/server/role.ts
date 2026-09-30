import { and, type Column, eq, type Table } from "@destack/db";
import { Scope } from "@destack/sync";
import { Snapshot } from "@destack/db/log";
import type { ObjectType } from "@destack/object";
import type { Stack } from "@destack/object/server";
import { identifier, type schema, type Identifier } from "@destack/schema";
import {
    accessRolePermission,
    permissionKey,
    principal,
    type PermissionReference,
    Role,
    type Subject,
} from "@destack/access";
import { group } from "@destack/account/object";
import type {
    SpaceObjectReference,
    SpaceRelationship,
    SpaceRoleReference,
} from "../declare/permission.ts";
import { SpaceError } from "../error/index.ts";
import * as base from "../object/index.ts";
import { installation } from "../object/index.ts";

/** Roles a stack declares in its space, after every object their permissions may name. */
export const role = base.role.declare({
    after: "every",
    values: (name, resolved) => ({
        name,
        description: resolved.description,
    }),
    changed: async (database, row, resolved) => {
        const current = await database
            .select({
                packageId: accessRolePermission.packageId,
                type: accessRolePermission.type,
                name: accessRolePermission.name,
            })
            .from(accessRolePermission)
            .where(eq(accessRolePermission.roleId, row.id));

        return (
            JSON.stringify(sortPermissions(current)) !==
            JSON.stringify(sortPermissions(resolved.permissions))
        );
    },
    written: (stack, row, resolved) =>
        Role.replace(stack.database, row.id, row.scope, resolved.permissions),
});

/** Relationships a stack declares in its space, with objects, roles and subjects resolved. */
export const relationship = base.relationship.declare({
    after: "every",
    resolve: async (_name, declared, stack) => {
        // relate the declared object, or the space itself as its account contains it, living in the space either way
        const spaceId = identifier("space").parse(stack.scope);
        const object =
            declared.object === undefined
                ? await Scope.object(Snapshot.live(stack.database), spaceId)
                : await objectReference(declared.object, stack);

        // require a relationship access accepts: one relation or role, a subject it takes, and a future expiry
        const relation = "relation" in declared ? declared.relation : undefined;
        const role = "role" in declared ? await roleIdentifier(declared.role, stack) : undefined;
        const subject = await subjectReference(declared.subject, spaceId, stack);
        stack.authorizer.validate(
            {
                object,
                ...(relation === undefined ? {} : { relation }),
                ...(role === undefined ? {} : { role }),
                subject,
                expiresAt: declared.expiresAt ?? null,
            },
            stack.now,
        );

        return {
            scope: spaceId,
            objectScope: object.scope,
            packageId: object.packageId,
            type: object.type,
            objectId: object.id,
            relation: relation ?? null,
            roleId: role ?? null,
            ...subjectColumns(subject),
            expiresAt: declared.expiresAt ?? null,
        };
    },
    values: (_name, resolved) => resolved,
});

/** Reference an object the stack declares or selects by identifier. */
async function objectReference(reference: schema.Infer<typeof SpaceObjectReference>, stack: Stack) {
    // find the type by the declaring collection's object or by the identifier's prefix
    const kind = "id" in reference ? reference.id.replace(/-[0-9a-f-]+$/, "") : reference.kind;
    const type = stack.object(kind);
    const id =
        "id" in reference
            ? await requireIn(type, reference.id, stack)
            : await stack.require(type, reference.name);

    return type.reference(stack.scope, id);
}

/** Resolve a role the stack declares or selects by identifier. */
async function roleIdentifier(
    reference: schema.Infer<typeof SpaceRoleReference>,
    stack: Stack,
): Promise<Identifier<"role">> {
    return identifier("role").parse(
        typeof reference === "string"
            ? await stack.require(base.role, reference)
            : await requireIn(base.role, reference.id, stack),
    );
}

/** Resolve a declared subject to the principal, subject set or role it refers to. */
async function subjectReference(
    subject: schema.Infer<typeof SpaceRelationship>["subject"],
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
        return base.role.reference(spaceId, await roleIdentifier(subject.role, stack));
    }

    // relate members and service accounts of the account containing the space
    const accountId = (await Scope.object(Snapshot.live(stack.database), spaceId)).scope;
    // relate a group's members
    if ("group" in subject) {
        return { ...group.reference(accountId, subject.group), relation: "member" };
    }
    // relate the account's members
    else if ("accountMembers" in subject) {
        return {
            ...(await Scope.object(Snapshot.live(stack.database), accountId)),
            relation: "member",
        };
    }
    // relate a service account of the account
    else {
        return principal.serviceAccount.reference(accountId, subject.serviceAccount);
    }
}

/** Require an object of a type by identifier in the stack's space, refusing one elsewhere or absent. */
async function requireIn(type: ObjectType, id: string, stack: Stack): Promise<string> {
    // find the object in the space
    const table = type.table as Table & Record<"id" | "scope", Column>;
    const [found] = await stack.database
        .select({ id: table.id })
        .from(table)
        .where(and(eq(table.id, id), eq(table.scope, stack.scope)));
    if (found === undefined) {
        throw new SpaceError("INVALID_DEFINITION", `the space has no ${type.name} ${id}`);
    }

    return id;
}

/** Spell a subject as its relationship columns. */
function subjectColumns(subject: Subject) {
    return {
        subjectPackageId: subject.packageId,
        subjectType: subject.type,
        subjectScope: subject.scope,
        subjectId: subject.id,
        subjectRelation: subject.relation ?? null,
    };
}

/** Order permissions for comparison independently of declaration order. */
function sortPermissions(
    permissions: readonly Pick<PermissionReference, "packageId" | "type" | "name">[],
) {
    return permissions.map(permissionKey).sort();
}
