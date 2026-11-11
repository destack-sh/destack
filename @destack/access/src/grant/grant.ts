import { Table } from "@destack/db";
import { Caller } from "../context/context.ts";
import { isPrincipal, principal } from "../declare/principal.ts";
import type { Authorization } from "../authorizer/authorization.ts";
import type { Select, Snapshot } from "@destack/db";
import { Identifier, Instant, nth, schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import type { AccessContext, GrantClaim } from "../context/context.ts";
import { GrantPermission } from "./permission.ts";
import { accessGrant } from "./table.ts";

/** The schema of what a delegator grants: delegate, permissions and lapse. */
const grantInputSchema = schema.object({
    /** The principal acting for the delegator. */
    delegate: Subject,
    /** The permissions the delegate may use for the delegator. */
    permissions: schema.array(GrantPermission.schema),
    /** The exclusive time the grant lapses, in Unix milliseconds, absent for none. */
    expiresAt: Instant.exactOptional(),
    /** The delegator's session the grant ends with, absent for none. */
    sessionId: schema.identifier("session").exactOptional(),
});

/** A delegator's grant of permissions to a delegate acting for it. */
export type Grant = Select<typeof accessGrant>;

/** A grant: its token claim, and its liveness for a caller. */
export const Grant = {
    /** The schema of what a delegator grants: delegate, permissions and lapse. */
    input: grantInputSchema,

    /** Read the id of the space a delegate acts in: its own or its installation's, else none. */
    spaceId(delegate: Subject): string | null {
        return principal.space.is(delegate)
            ? delegate.id
            : principal.installation.is(delegate)
              ? delegate.scope
              : null;
    },

    /** Read the claim a token includes for a grant. */
    claim(grant: Grant): GrantClaim {
        return { id: grant.id, scope: grant.scope, permissions: grant.permissions };
    },

    /** Read a grant by identifier as a snapshot shows it, absent once revoked. */
    async read(
        reader: Pick<Snapshot, "read">,
        id: Identifier<"grant">,
    ): Promise<Grant | undefined> {
        return reader.read(accessGrant).findFirst({ where: { id } });
    },

    /** Read what a delegated caller may use: its chain's signed grants intersected. */
    permissions(context: AccessContext): GrantPermission[] | undefined {
        // intersect what each link's grant gives
        let given: GrantPermission[] | undefined;
        for (const { grant, delegator } of Caller.links(context)) {
            // use a link's approval-mode permissions once its delegator approved the call's change
            const isApproved = (context.approvers ?? []).some((each) =>
                Subject.same(each, delegator),
            );
            const usable = isApproved
                ? grant.permissions.map(({ mode: _mode, ...entry }) => entry)
                : grant.permissions;
            given = GrantPermission.intersect(given, usable);
        }

        return given;
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
    act(
        reader: Pick<Snapshot, "read">,
        parties: { readonly delegator: Subject; readonly delegate: Subject },
        now: number,
    ): Promise<Caller> {
        return Grant.chain(reader, { subject: parties.delegator, actors: [parties.delegate] }, now);
    },

    /** Act for a subject through a chain of actors, each within the prior one's live grant. */
    async chain(
        reader: Pick<Snapshot, "read">,
        chain: { readonly subject: Subject; readonly actors: readonly Subject[] },
        now: number,
    ): Promise<Caller> {
        // read each link's live grant, refusing a link without one
        const grants: GrantClaim[] = [];
        for (const [index, delegate] of chain.actors.entries()) {
            const delegator = index === 0 ? chain.subject : nth(chain.actors, index - 1);
            const live = await Grant.find(reader, { delegator, delegate }, now);
            if (live === undefined) {
                throw AccessError.of(
                    "GRANT_DENIED",
                    `${delegator.type} '${delegator.id}' is forbidden: ${delegate.type} '${delegate.id}' has no grant to act for it`,
                );
            }
            grants.push(Grant.claim(live));
        }

        // act as the subject with the actors acting for it, within every grant of the chain
        return { ...Caller.of(chain.subject), actors: [...chain.actors], grants };
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
            throw AccessError.of(
                "GRANT_UNAUTHENTICATED",
                `authentication required: grant '${id}' is revoked`,
            );
        } else if (grant.expiresAt !== null && grant.expiresAt <= now) {
            throw AccessError.of(
                "GRANT_UNAUTHENTICATED",
                `authentication required: grant '${id}' expired`,
            );
        }

        // refuse a grant of another delegator or to another delegate
        if (
            !Subject.same(ObjectReference.at("delegator", grant), parties.delegator) ||
            !Subject.same(ObjectReference.at("delegate", grant), parties.delegate)
        ) {
            throw AccessError.of(
                "GRANT_DELEGATE_UNAUTHENTICATED",
                `authentication required: grant '${id}' belongs to another delegate`,
            );
        }

        return grant;
    },

    /** Grant a delegate some of the caller's authority in its scope, replacing an earlier grant. */
    async create(
        scope: ObjectReference,
        input: schema.Infer<typeof grantInputSchema>,
        authorization: Authorization,
    ): Promise<Grant> {
        // write only the grants of a homed scope
        await authorization.authorizer.requireHome(authorization.transaction, scope, scope.id);

        // grant only a principal's authority, in its scope
        const context = authorization.context();
        const delegator = Caller.principal(context);
        if (delegator === undefined || Caller.isDelegated(context) || delegator.id !== scope.id) {
            throw AccessError.of(
                "AUTHORITY_DENIED",
                "authority is forbidden: a principal can grant only its own authority",
            );
        }

        // grant to one other principal, until a later time
        const { delegate } = input;
        if (
            !isPrincipal(delegate) ||
            delegate.id === "*" ||
            delegate.scope === "*" ||
            Subject.same(delegate, delegator)
        ) {
            throw AccessError.of(
                "DELEGATE_DENIED",
                "grant is forbidden: it needs one other principal as delegate",
            );
        } else if (input.expiresAt !== undefined && input.expiresAt <= context.now) {
            throw AccessError.of(
                "GRANT_EXPIRY_INVALID",
                "grant expiry is invalid: it must be after its creation",
            );
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
            id: Identifier.create("grant", authorization.environment),
            createdAt: context.now,
            scope: scope.id,
            ...ObjectReference.columns("delegator", delegator),
            ...ObjectReference.columns("delegate", delegate),
            delegateSpaceId: Grant.spaceId(delegate),
            permissions: [...input.permissions],
            expiresAt: input.expiresAt ?? null,
            sessionId: input.sessionId ?? null,
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
        // write only the grants of a homed scope
        await authorization.authorizer.requireHome(authorization.transaction, scope, scope.id);

        // require the grant in the scope and its delegator acting as itself
        const context = authorization.context();
        const delegator = Caller.principal(context);
        const grant = await Grant.read(authorization.snapshot, id);
        if (grant === undefined || grant.scope !== scope.id) {
            throw AccessError.of("GRANT_NOT_FOUND", `grant '${id}' not found`);
        } else if (
            delegator === undefined ||
            Caller.isDelegated(context) ||
            !Subject.same(delegator, ObjectReference.at("delegator", grant))
        ) {
            throw AccessError.of(
                "DELEGATOR_DENIED",
                "grant revocation is forbidden: only the delegator can revoke a grant",
            );
        }

        // delete it
        await authorization.transaction.write({ table: accessGrant, before: grant, after: null });

        return grant;
    },
};
