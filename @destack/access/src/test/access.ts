import { eq, type DatabaseConnection } from "@destack/db";
import { Scope, type ObjectReference, type Subject } from "@destack/sync";
import { schema } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import { own, permit, type RoleRequest } from "../role/role.ts";
import { accessRole } from "../role/table.ts";

/** A database keeping copies of other services' access, for tests of services that follow one. */
export class AccessFixture {
    /** The database keeping the copies. */
    readonly database: DatabaseConnection;

    /** Keep copies of access in a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Record a scope object below the scopes containing it, as a copy of its home's access keeps it. */
    async copyScope(scope: ObjectReference): Promise<void> {
        // read the ancestry of the containing scope
        const [parent] =
            scope.scope === Scope.universe.id
                ? [{ ancestors: [] }]
                : await this.database
                      .select({ ancestors: Scope.table.ancestors })
                      .from(Scope.table)
                      .where(eq(Scope.table.scope, scope.scope));
        if (parent === undefined) {
            throw new AccessError("NOT_FOUND", `unknown scope: ${scope.scope}`);
        }

        // record the scope below it
        await this.database.insert(Scope.table).values({
            scope: scope.id,
            parent: scope.scope,
            packageId: scope.packageId,
            type: scope.type,
            ancestors: scope.scope === Scope.universe.id ? [] : [scope.scope, ...parent.ancestors],
        });
    }

    /** Define a copied scope's owner role and bind it to an owner on the scope's object, returning the role. */
    copyOwner(scope: ObjectReference, owner: Subject, now = Date.now()): Promise<string> {
        return own(this.database, scope, owner, now);
    }

    /** Define a role in a copied scope and bind it to a subject on the scope's object, returning the role. */
    async copyRole(
        scope: ObjectReference,
        role: RoleRequest,
        subject: Subject,
        now = Date.now(),
    ): Promise<string> {
        // define the role in the scope
        const id = schema.identifier("role").parse(`role-${v7()}`);
        await this.database.insert(accessRole).values({
            id,
            createdAt: now,
            updatedAt: now,
            scope: scope.id,
            name: role.name,
            description: role.description,
        });
        await permit(this.database, id, scope.id, role.permissions);

        // bind it on the scope's object
        await this.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
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
    async suspendCopy(scope: string, suspendedAt: number | null): Promise<void> {
        await this.database
            .update(Scope.table)
            .set({ suspendedAt })
            .where(eq(Scope.table.scope, scope));
    }
}
