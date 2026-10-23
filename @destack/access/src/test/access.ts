import { and, eq, Table, Transaction, type DatabaseConnection } from "@destack/db";
import { ObjectReference, Scope, type Subject } from "@destack/sync";
import { schema, type Identifier } from "@destack/schema";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import type { GrantPermission } from "../grant/permission.ts";
import type { Policy } from "../declare/policy.ts";
import { accessGrant } from "../grant/table.ts";
import { Role, type RoleRequest } from "../role/role.ts";

/** A database storing replicas of other packages' access, for tests of packages that follow one. */
export class AccessFixture {
    /** The database storing the replicas. */
    readonly database: DatabaseConnection;

    /** Store replicas of access in a database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Record a scope below its containing scopes, partitioned as its type's policy declares. */
    async replicateScope(scope: ObjectReference, policy: Policy): Promise<void> {
        // read the ancestry and partition of the containing scope
        const [parent] =
            scope.scope === Scope.universe.id
                ? [{ ancestors: [], partition: Scope.universe.id }]
                : await this.database
                      .select({
                          ancestors: Scope.table.ancestors,
                          partition: Scope.table.partition,
                      })
                      .from(Scope.table)
                      .where(eq(Scope.table.scope, scope.scope));
        if (parent === undefined) {
            throw AccessError.of("SCOPE_NOT_FOUND", `unknown scope: ${scope.scope}`);
        }

        // record the scope below it
        await this.database.insert(Scope.table).values({
            scope: scope.id,
            parent: scope.scope,
            partition: policy.definition.partition === true ? scope.id : parent.partition,
            packageId: scope.packageId,
            type: scope.type,
            ancestors: scope.scope === Scope.universe.id ? [] : [scope.scope, ...parent.ancestors],
        });
    }

    /** Bind the owner role on a scope to an owner, as its home does. */
    async bindOwner(scope: ObjectReference, owner: Subject, now = Date.now()): Promise<void> {
        // bind the owner role in one transaction
        const transaction = await Transaction.open(this.database);
        await Role.bindOwner(transaction, scope, owner, now);
        await transaction.commit();
    }

    /** Define a role in a replicated scope and bind it to a subject on the scope, returning it. */
    async replicateRole(
        scope: ObjectReference,
        role: RoleRequest,
        subject: Subject,
        now = Date.now(),
    ): Promise<string> {
        // define the role with its permissions and bind it in one transaction
        const transaction = await Transaction.open(this.database);
        const id = await Role.upsertInherent(transaction, scope, role, subject, now);
        await transaction.commit();

        return id;
    }

    /** Relate a subject to an object in its scope, as a home access replica does. */
    async replicateRelationship(
        object: ObjectReference,
        relation: string,
        subject: Subject,
        now = Date.now(),
    ): Promise<Identifier<"relationship">> {
        const id = schema.identifier("relationship").parse(`relationship-${v7()}`);
        await this.database.insert(accessRelationship).values(
            Relationship.encode({
                id,
                object,
                relation,
                subject,
                createdAt: now,
                expiresAt: null,
            }),
        );

        return id;
    }

    /** Grant a delegate some of a principal's authority in its scope, as its home does. */
    async replicateGrant(
        delegator: Subject,
        delegate: Subject,
        permissions: readonly GrantPermission[],
        now = Date.now(),
        id = schema.identifier("grant").parse(`grant-${v7()}`),
    ): Promise<Identifier<"grant">> {
        // replace the parties' earlier grant, as the two have one grant at a time
        await this.database
            .delete(accessGrant)
            .where(
                and(
                    eq(accessGrant.delegatorType, delegator.type),
                    eq(accessGrant.delegatorId, delegator.id),
                    eq(accessGrant.delegateType, delegate.type),
                    eq(accessGrant.delegateId, delegate.id),
                ),
            );
        await this.database.insert(accessGrant).values(
            Table.complete(accessGrant, {
                id,
                createdAt: now,
                scope: delegator.id,
                ...ObjectReference.columns("delegator", delegator),
                ...ObjectReference.columns("delegate", delegate),
                permissions: [...permissions],
                expiresAt: null,
            }),
        );

        return id;
    }

    /** Mark a replicated scope suspended or active again, as its home's suspension replicates. */
    async suspendReplica(scope: string, suspendedAt: number | null): Promise<void> {
        await this.database
            .update(Scope.table)
            .set({ suspendedAt })
            .where(eq(Scope.table.scope, scope));
    }
}
