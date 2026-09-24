import { and, eq, sql, TABLE, type DatabaseConnection, type Select } from "@destack/db";
import { v7 } from "uuid";
import { AccessError } from "../error/index.ts";
import { type Grant, isActive } from "../grant/grant.ts";
import { createTokenCredential, digestTokenCredential } from "../grant/token.ts";
import type { ObjectReference } from "../access/index.ts";
import type { AccessContext, Subject } from "../access/subject.ts";
import { AccessQuery } from "./query.ts";
import { accessGrant, accessToken } from "../stack/schema.ts";

/** Persist sharing with explicit transactional authorization and audit recording. */
export class GrantStore {
    /** Database storing access grants. */
    readonly database: DatabaseConnection;
    /** Access query compiled for this database. */
    readonly query: AccessQuery;
    /** Record grant changes in the same transaction. */
    readonly record: (
        change: GrantChange,
        context: AccessContext,
        transaction: DatabaseConnection,
    ) => Promise<void>;

    /** Bind persistence and the host's transactional audit implementation. */
    constructor(
        database: DatabaseConnection,
        query: AccessQuery,
        record: (
            change: GrantChange,
            context: AccessContext,
            transaction: DatabaseConnection,
        ) => Promise<void>,
    ) {
        this.database = database;
        this.query = query;
        this.record = record;
    }

    /** Authorize and persist an explicit sharing grant. */
    async grant(
        request: {
            object: ObjectReference;
            relation: string;
            subject: Grant["subject"];
            expiresAt?: number;
        },
        context: AccessContext,
    ): Promise<string> {
        return this.database.transaction(
            async (transaction) => {
                // require permission to administer the relation
                await this.#authorize(request.object, request.relation, context, transaction);

                // persist and audit together so an audit failure rolls back the grant
                const id = await this.#insert(request, context, transaction);
                await this.record(
                    {
                        operation: "grant",
                        grant: {
                            ...request,
                            id,
                            createdAt: context.now,
                            expiresAt: request.expiresAt ?? null,
                            revokedAt: null,
                        },
                    },
                    context,
                    transaction,
                );

                return id;
            },
            { isolationLevel: "serializable" },
        );
    }

    /** Create a bearer credential and its object-scoped grant atomically. */
    async createToken(
        request: { object: ObjectReference; relation: string; expiresAt?: number },
        context: AccessContext,
    ): Promise<{ id: string; secret: string }> {
        // create the credential before opening the write transaction
        const credential = await createTokenCredential();
        const id = `share-token-${v7()}`;

        // authorize before publishing either the token or its grant
        await this.database.transaction(
            async (transaction) => {
                // require permission to administer the relation
                await this.#authorize(request.object, request.relation, context, transaction);

                // store only the credential digest and its lifetime
                await transaction.insert(accessToken).values({
                    id,
                    scope: request.object.scope,
                    digest: credential.digest,
                    createdAt: context.now,
                    expiresAt: request.expiresAt ?? null,
                    revokedAt: null,
                });

                // grant the token access to the authorized object
                const grantId = await this.#insert(
                    {
                        ...request,
                        subject: { kind: "share-token", authority: request.object.scope, id },
                    },
                    context,
                    transaction,
                );

                // commit the token, grant and audit record together
                await this.record(
                    {
                        operation: "create-token",
                        tokenId: id,
                        grant: {
                            ...request,
                            id: grantId,
                            subject: { kind: "share-token", authority: request.object.scope, id },
                            createdAt: context.now,
                            expiresAt: request.expiresAt ?? null,
                            revokedAt: null,
                        },
                    },
                    context,
                    transaction,
                );
            },
            { isolationLevel: "serializable" },
        );

        return { id, secret: credential.secret };
    }

    /** Verify possession; authorization still checks the token's current persisted state. */
    async authenticateToken(secret: string, now: number): Promise<Subject> {
        // find the persisted credential by its digest
        const digest = await digestTokenCredential(secret);
        const [token] = await this.database
            .select()
            .from(accessToken)
            .where(eq(accessToken.digest, digest));

        // reject revoked, expired or not yet active credentials
        if (!token || !isActive(token, now) || !Number.isFinite(now)) {
            throw new AccessError("FORBIDDEN", "invalid or expired share credential");
        }

        return { kind: "share-token", authority: token.scope, id: token.id };
    }

    /** Revoke one grant without changing unrelated permissions. */
    async revoke(id: string, context: AccessContext): Promise<void> {
        await this.database.transaction(
            async (transaction) => {
                // load the grant inside the authorization transaction
                const [grant] = await transaction
                    .select()
                    .from(accessGrant)
                    .where(eq(accessGrant.id, id));
                if (!grant) {
                    throw new AccessError("NOT_FOUND", "grant not found");
                }

                // authorize the grant's persisted object and relation
                const object = {
                    packageId: grant.packageId,
                    type: grant.type,
                    scope: grant.scope,
                    id: grant.objectId,
                };
                await this.#authorize(object, grant.relation, context, transaction);

                // retain the first revocation time and record the request atomically
                await transaction
                    .update(accessGrant)
                    .set({ revokedAt: context.now })
                    .where(and(eq(accessGrant.id, id), sql`${accessGrant.revokedAt} IS NULL`));
                await this.record(
                    {
                        operation: "revoke",
                        grant: describeGrant({
                            ...grant,
                            revokedAt: grant.revokedAt ?? context.now,
                        }),
                    },
                    context,
                    transaction,
                );
            },
            { isolationLevel: "serializable" },
        );
    }

    /** Revoke a bearer token after authorizing every object it grants access to. */
    async revokeToken(id: string, context: AccessContext): Promise<void> {
        await this.database.transaction(
            async (transaction) => {
                // resolve the token's authority before loading its grants
                const [token] = await transaction
                    .select()
                    .from(accessToken)
                    .where(eq(accessToken.id, id));
                if (!token) {
                    throw new AccessError("NOT_FOUND", "share token not found");
                }

                // read every grant belonging to this token and authority
                const grants = await transaction
                    .select()
                    .from(accessGrant)
                    .where(
                        and(
                            eq(accessGrant.subjectKind, "share-token"),
                            eq(accessGrant.subjectId, id),
                            eq(accessGrant.subjectAuthority, token.scope),
                        ),
                    );

                // require an object through which this token can be administered and audited
                if (grants.length === 0) {
                    throw new AccessError("CONFLICT", "share token has no grants");
                }

                // authorize and audit each affected object before revoking the token
                for (const grant of grants) {
                    const object = {
                        packageId: grant.packageId,
                        type: grant.type,
                        scope: grant.scope,
                        id: grant.objectId,
                    };
                    await this.#authorize(object, grant.relation, context, transaction);
                    await this.record(
                        { operation: "revoke-token", tokenId: id, grant: describeGrant(grant) },
                        context,
                        transaction,
                    );
                }

                // retain the first revocation time across repeated requests
                await transaction
                    .update(accessToken)
                    .set({ revokedAt: context.now })
                    .where(and(eq(accessToken.id, id), sql`${accessToken.revokedAt} IS NULL`));
            },
            { isolationLevel: "serializable" },
        );
    }

    /** Check the relation's administrative permission in the mutation transaction. */
    async #authorize(
        object: ObjectReference,
        name: string,
        context: AccessContext,
        transaction: DatabaseConnection,
    ): Promise<void> {
        // resolve the declared permission required to administer this relation
        const mapping = this.query.mapping(object);
        const relation = mapping.type.definition.relations[name];
        if (!Object.hasOwn(mapping.type.definition.relations, name) || relation.kind !== "grant") {
            throw new AccessError("FORBIDDEN", "relation cannot be granted");
        }

        // evaluate permission against the application row inside this transaction
        const predicate = this.query.where(
            mapping.type.permission(relation.permission),
            object.scope,
            context,
        );
        const id = mapping.table[TABLE].columns[mapping.id];
        const rows = await transaction
            .select({ id })
            .from(mapping.table)
            .where(and(predicate, sql`${id} = ${object.id}`))
            .limit(1);

        // require an authorized matching row before changing any grants
        if (rows.length === 0) {
            throw new AccessError("FORBIDDEN", "sharing permission denied");
        }
    }

    /** Persist a subject kind explicitly accepted by the relation declaration. */
    async #insert(
        request: {
            object: ObjectReference;
            relation: string;
            subject: Grant["subject"];
            expiresAt?: number;
        },
        context: AccessContext,
        transaction: DatabaseConnection,
    ): Promise<string> {
        // require a declared subject kind and a valid future expiry
        const definition = this.query.model.type(request.object).definition.relations[
            request.relation
        ];
        if (
            definition.kind !== "grant" ||
            !definition.subjects.includes(request.subject.kind) ||
            (request.expiresAt !== undefined &&
                (!Number.isFinite(request.expiresAt) || request.expiresAt <= context.now))
        ) {
            throw new AccessError("FORBIDDEN", "invalid grant subject or expiry");
        }

        // persist the grant with a stable identifier for audit and revocation
        const id = `grant-${v7()}`;
        const subject = request.subject;
        await transaction.insert(accessGrant).values({
            id,
            packageId: request.object.packageId,
            type: request.object.type,
            scope: request.object.scope,
            objectId: request.object.id,
            relation: request.relation,
            subjectKind: subject.kind,
            subjectAuthority: subject.kind === "everyone" ? "" : subject.authority,
            subjectId: subject.kind === "everyone" ? "" : subject.id,
            createdAt: context.now,
            expiresAt: request.expiresAt ?? null,
            revokedAt: null,
        });

        return id;
    }
}

/** A grant change recorded within the same transaction as application authorization. */
export type GrantChange = {
    /** The affected grant, including its recipient and lifetime. */
    readonly grant: Grant;
} & (
    | { readonly operation: "grant" | "revoke" }
    | { readonly operation: "create-token" | "revoke-token"; readonly tokenId: string }
);

/** Describe the grant persisted in an application database. */
function describeGrant(record: Select<typeof accessGrant>): Grant {
    return {
        id: record.id,
        object: {
            packageId: record.packageId,
            type: record.type,
            scope: record.scope,
            id: record.objectId,
        },
        relation: record.relation,
        subject:
            record.subjectKind === "everyone"
                ? { kind: "everyone" }
                : {
                      kind: record.subjectKind,
                      authority: record.subjectAuthority,
                      id: record.subjectId,
                  },
        createdAt: record.createdAt,
        expiresAt: record.expiresAt,
        revokedAt: record.revokedAt,
    };
}
