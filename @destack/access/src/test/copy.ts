import { eq, type DatabaseConnection } from "@destack/db";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { GLOBAL_SCOPE } from "../context/context.ts";
import { AccessError } from "../error/index.ts";
import type { ObjectReference } from "../policy/policy.ts";
import type { Subject } from "../policy/subject.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { Role, type RoleRequest } from "../role/role.ts";
import { accessRole } from "../role/table.ts";
import { accessScope } from "../scope/table.ts";

/** Record a scope object below the scopes containing it, as a copy of its home's access holds it. */
export async function copyScope(
    database: DatabaseConnection,
    scope: ObjectReference,
): Promise<void> {
    // read the ancestry of the containing scope, which a copy holds before the scopes below it
    const [parent] =
        scope.scope === GLOBAL_SCOPE
            ? [{ ancestors: [] }]
            : await database
                  .select({ ancestors: accessScope.ancestors })
                  .from(accessScope)
                  .where(eq(accessScope.scope, scope.scope));
    if (parent === undefined) {
        throw new AccessError("NOT_FOUND", `unknown scope: ${scope.scope}`);
    }

    // record the scope below it
    await database.insert(accessScope).values({
        scope: scope.id,
        parent: scope.scope,
        packageId: scope.packageId,
        type: scope.type,
        ancestors: scope.scope === GLOBAL_SCOPE ? [] : [scope.scope, ...parent.ancestors],
    });
}

/** Define a copied scope's owner role and bind it to an owner on the scope's object, returning the role. */
export function copyOwner(
    database: DatabaseConnection,
    scope: ObjectReference,
    owner: Subject,
    now = Date.now(),
): Promise<string> {
    return Role.own(database, scope, owner, now);
}

/** Define a role in a copied scope and bind it to a subject on the scope's object, returning the role. */
export async function copyRole(
    database: DatabaseConnection,
    scope: ObjectReference,
    role: RoleRequest,
    subject: Subject,
    now = Date.now(),
): Promise<string> {
    // define the role in the scope
    const id = identifier("role").parse(`role-${v7()}`);
    await database.insert(accessRole).values({
        id,
        createdAt: now,
        updatedAt: now,
        scope: scope.id,
        name: role.name,
        description: role.description,
    });
    await Role.permit(database, id, scope.id, role.permissions);

    // bind it on the scope's object
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: identifier("relationship").parse(`relationship-${v7()}`),
                object: scope,
                role: id,
                subject,
                createdAt: now,
                expiresAt: null,
            },
            scope.id,
        ),
    );

    return id;
}

/** Mark a copied scope suspended, or active again, as its home's suspension arrives by replication. */
export async function suspendCopy(
    database: DatabaseConnection,
    scope: string,
    suspendedAt: number | null,
): Promise<void> {
    await database.update(accessScope).set({ suspendedAt }).where(eq(accessScope.scope, scope));
}
