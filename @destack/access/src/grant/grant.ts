import { v7 } from "uuid";
import { Table } from "@destack/db";
import { Caller } from "../context/context.ts";
import { isPrincipal } from "../declare/principal.ts";
import type { Authorization } from "../authorizer/authorization.ts";
import type { Select, Snapshot } from "@destack/db";
import { defineSchema, type Identifier, Instant, schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import type { GrantClaim } from "../context/context.ts";
import { GrantPermission } from "./permission.ts";
import { accessGrant } from "./table.ts";

/** What a delegator grants: delegate, permissions and lapse. */
export const GrantRequest = defineSchema(
    schema.object({
        /** The principal acting for the delegator. */
        delegate: Subject,
        /** The permissions the delegate may use for the delegator. */
        permissions: schema.array(GrantPermission.schema),
        /** The exclusive time the grant lapses, in Unix milliseconds, absent for none. */
        expiresAt: Instant.exactOptional(),
    }),
);
/** What a delegator grants. */
export type GrantRequest = schema.Infer<typeof GrantRequest>;

/** A delegator's grant of permissions to a delegate acting for it. */
export type Grant = Select<typeof accessGrant>;

/** A grant: its token claim, and its liveness for a caller. */
export const Grant = {
    /** Read the claim a token carries for a grant. */
    claim(grant: Grant): GrantClaim {
        return { id: grant.id, scope: grant.scope, permissions: [...grant.permissions] };
    },

    /** Read a grant by identifier as a snapshot shows it, absent once revoked. */
    async read(snapshot: Snapshot, id: Identifier<"grant">): Promise<Grant | undefined> {
        return snapshot.read(accessGrant).findFirst({ where: { id } });
    },

    /** Find a delegator's one grant to a delegate unexpired at a time, absent without one. */
    async find(
        reader: Pick<Snapshot, "read">,
        parties: { readonly delegator: Subject; readonly delegate: Subject },
        now: number,
    ): Promise<Grant | undefined> {
        // read the grant of the two parties
        const grant = await reader.read(accessGrant).findFirst({
            where: {
                ...ObjectReference.columns("delegator", parties.delegator),
                ...ObjectReference.columns("delegate", parties.delegate),
            },
        });

        return grant === undefined || (grant.expiresAt !== null && grant.expiresAt <= now)
            ? undefined
            : grant;
    },

    /** Act for a delegator as its delegate within the delegator's live grant. */
    async act(
        reader: Pick<Snapshot, "read">,
        parties: { readonly delegator: Subject; readonly delegate: Subject },
        now: number,
    ): Promise<Caller> {
        // read the delegator's live grant to the delegate
        const { delegator, delegate } = parties;
        const live = await Grant.find(reader, parties, now);
        if (live === undefined) {
            throw new AccessError(
                "FORBIDDEN",
                `${delegate.type} ${delegate.id} acts for ${delegator.type} ${delegator.id} under no grant`,
            );
        }

        // act as the delegator with the delegate acting for it, within the grant
        return { ...Caller.of(delegator), actors: [delegate], grant: Grant.claim(live) };
    },

    /** Require a grant between a delegator and a delegate unrevoked and unexpired at a time. */
    require(
        grant: Grant | undefined,
        id: Identifier<"grant">,
        parties: { readonly delegator: Subject; readonly delegate: Subject },
        now: number,
    ): Grant {
        // refuse a revoked or lapsed grant
        if (grant === undefined) {
            throw new AccessError("UNAUTHORIZED", `grant ${id} is revoked`);
        } else if (grant.expiresAt !== null && grant.expiresAt <= now) {
            throw new AccessError("UNAUTHORIZED", `grant ${id} expired`);
        }

        // refuse a grant of another delegator or to another delegate
        if (
            !Subject.same(ObjectReference.at("delegator", grant), parties.delegator) ||
            !Subject.same(ObjectReference.at("delegate", grant), parties.delegate)
        ) {
            throw new AccessError("UNAUTHORIZED", `grant ${id} belongs to another delegate`);
        }

        return grant;
    },

    /** Grant a delegate part of the caller's authority in its scope, replacing an earlier grant. */
    async create(
        scope: ObjectReference,
        request: GrantRequest,
        authorization: Authorization,
    ): Promise<Grant> {
        // write only the grants of a local scope
        await authorization.authorizer.requireLocal(authorization.transaction, scope);

        // grant only a principal's own authority, in its own scope
        const context = authorization.context();
        const delegator = Caller.principal(context);
        if (delegator === undefined || Caller.isDelegated(context) || delegator.id !== scope.id) {
            throw new AccessError("FORBIDDEN", "a principal can grant only its own authority");
        }

        // grant to one other principal, until a later time
        const { delegate } = request;
        if (
            !isPrincipal(delegate) ||
            delegate.id === "*" ||
            delegate.scope === "*" ||
            Subject.same(delegate, delegator)
        ) {
            throw new AccessError("FORBIDDEN", "a grant needs one other principal as delegate");
        } else if (request.expiresAt !== undefined && request.expiresAt <= context.now) {
            throw new AccessError("CONFLICT", "a grant must expire after its creation");
        }

        // replace the delegator's earlier grant to the delegate, live or lapsed
        const [earlier] = await authorization.snapshot.read(accessGrant).findMany({
            where: {
                ...ObjectReference.columns("delegator", delegator),
                ...ObjectReference.columns("delegate", delegate),
            },
        });
        if (earlier !== undefined) {
            await authorization.transaction.write({
                table: accessGrant,
                before: earlier,
                after: null,
            });
        }

        // write the grant
        const grant = Table.complete(accessGrant, {
            id: schema.identifier("grant").parse(`grant-${v7()}`),
            createdAt: context.now,
            scope: scope.id,
            ...ObjectReference.columns("delegator", delegator),
            ...ObjectReference.columns("delegate", delegate),
            permissions: [...request.permissions],
            expiresAt: request.expiresAt ?? null,
        });
        await authorization.transaction.write({ table: accessGrant, before: null, after: grant });

        return grant;
    },

    /** Revoke a grant, as its delegator. */
    async revoke(
        scope: ObjectReference,
        id: Identifier<"grant">,
        authorization: Authorization,
    ): Promise<Grant> {
        // write only the grants of a local scope
        await authorization.authorizer.requireLocal(authorization.transaction, scope);

        // require the grant in the scope and its delegator acting as itself
        const context = authorization.context();
        const delegator = Caller.principal(context);
        const grant = await Grant.read(authorization.snapshot, id);
        if (grant === undefined || grant.scope !== scope.id) {
            throw new AccessError("NOT_FOUND", `no grant ${id}`);
        } else if (
            delegator === undefined ||
            Caller.isDelegated(context) ||
            !Subject.same(delegator, ObjectReference.at("delegator", grant))
        ) {
            throw new AccessError("FORBIDDEN", "only the delegator can revoke a grant");
        }

        // delete it
        await authorization.transaction.write({ table: accessGrant, before: grant, after: null });

        return grant;
    },
};
