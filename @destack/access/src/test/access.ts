import { and, eq, Table, Transaction, type DatabaseConnection } from "@destack/db";
import { ObjectReference, Scope, type Subject } from "@destack/sync";
import { Identifier, present } from "@destack/schema";
import { AccessError } from "../error/index.ts";
import { Relationship } from "../relationship/relationship.ts";
import { accessRelationship } from "../relationship/table.ts";
import type { GrantPermission } from "../grant/permission.ts";
import type { Policy } from "../declare/policy.ts";
import { accessGrant } from "../grant/table.ts";
import { Role, type RoleRequest } from "../role/role.ts";
import { AccessContext, Caller } from "../context/context.ts";
import { Authorization, type AuthorizationEnvironment } from "../authorizer/authorization.ts";
import { Authorizer } from "../authorizer/authorizer.ts";

/** A database storing replicas of other packages' access, for tests of packages that follow one. */
export class TestAccess {
    /** The database storing the replicas. */
    readonly database: DatabaseConnection;
    /** The clock, randomness and crypto the rows take their identifiers and digests from. */
    readonly environment: AuthorizationEnvironment;
    /** The partition whose scopes' access the database homes, none when absent. */
    readonly partition: string | undefined;

    /** Store access in a database homing a partition, identified from an environment. */
    constructor(
        database: DatabaseConnection,
        environment: AuthorizationEnvironment,
        partition?: string,
    ) {
        this.database = database;
        this.environment = environment;
        this.partition = partition;
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
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(Scope.table).values({
                    scope: scope.id,
                    parent: scope.scope,
                    partition: policy.definition.partition === true ? scope.id : parent.partition,
                    packageId: scope.packageId,
                    type: scope.type,
                    ancestors:
                        scope.scope === Scope.universe.id ? [] : [scope.scope, ...parent.ancestors],
                }),
        );
    }

    /** Bind the owner role on a scope to an owner, as its home does. */
    async bindOwner(
        scope: ObjectReference,
        owner: Subject,
        now = this.environment.clock.now(),
    ): Promise<void> {
        // bind the owner role as the owner, refused outside the binding's home
        const authorizer = new Authorizer(
            [],
            [],
            this.partition === undefined ? {} : { partition: this.partition },
        );
        const context = () => AccessContext.of(Caller.of(owner), now);
        await Authorization.run(
            authorizer,
            this.database,
            context,
            this.environment,
            (authorization) => Role.bind(scope, Role.OWNER, owner, authorization),
        );
    }

    /** Bind the owner role on a replicated scope, as its parent's primary replicates it. */
    async replicateOwner(
        scope: ObjectReference,
        owner: Subject,
        now = this.environment.clock.now(),
    ): Promise<Identifier<"relationship">> {
        const id = Identifier.create("relationship", this.environment);
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(accessRelationship).values(
                    Relationship.encode({
                        id,
                        object: scope,
                        role: Role.OWNER,
                        subject: owner,
                        createdAt: now,
                        expiresAt: null,
                    }),
                ),
        );

        return id;
    }

    /** Define a role in a replicated scope and bind it to a subject on the scope, returning it. */
    async replicateRole(
        scope: ObjectReference,
        role: RoleRequest,
        subject: Subject,
        now = this.environment.clock.now(),
    ): Promise<string> {
        // define the role with its permissions and bind it inside the scope, as the replica delivers them
        const transaction = await Transaction.open(this.database);
        const id = present(
            await Role.define(transaction, scope.id, role, now, null, this.environment),
            `the role ${role.name}`,
        );
        const relationship = Relationship.encode(
            {
                id: Identifier.create("relationship", this.environment),
                object: scope,
                role: id,
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
        await transaction.commit();

        return id;
    }

    /** Relate a subject to an object in its scope, as a home access replica does. */
    async replicateRelationship(
        object: ObjectReference,
        relation: string,
        subject: Subject,
        now = this.environment.clock.now(),
    ): Promise<Identifier<"relationship">> {
        const id = Identifier.create("relationship", this.environment);
        await this.database.transaction(
            async (transaction) =>
                await transaction.insert(accessRelationship).values(
                    Relationship.encode({
                        id,
                        object,
                        relation,
                        subject,
                        createdAt: now,
                        expiresAt: null,
                    }),
                ),
        );

        return id;
    }

    /** Grant a delegate some of a principal's authority in its scope, as its home does. */
    async replicateGrant(
        delegator: Subject,
        delegate: Subject,
        permissions: readonly GrantPermission[],
        now = this.environment.clock.now(),
        id = Identifier.create("grant", this.environment),
    ): Promise<Identifier<"grant">> {
        // replace the parties' earlier grant in one transaction
        await this.database.transaction(async (transaction) => {
            await transaction
                .delete(accessGrant)
                .where(
                    and(
                        eq(accessGrant.delegatorType, delegator.type),
                        eq(accessGrant.delegatorId, delegator.id),
                        eq(accessGrant.delegateType, delegate.type),
                        eq(accessGrant.delegateId, delegate.id),
                    ),
                );
            await transaction.insert(accessGrant).values(
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
        });

        return id;
    }

    /** Mark a replicated scope suspended or active again, as its home's suspension replicates. */
    async suspendReplica(scope: string, suspendedAt: number | null): Promise<void> {
        await this.database.transaction(
            async (transaction) =>
                await transaction
                    .update(Scope.table)
                    .set({ suspendedAt })
                    .where(eq(Scope.table.scope, scope)),
        );
    }
}
