import type { Select, Snapshot } from "@destack/db";
import { defineSchema, type Identifier, Instant, schema } from "@destack/schema";
import { Subject } from "@destack/sync";
import { AccessError } from "../error/index.ts";
import type { CallerGrant } from "../context/context.ts";
import { GrantPermission } from "./permission.ts";
import { accessGrant, GrantApproval } from "./table.ts";

/** What a delegator grants: delegate, permissions, approval and lapse. */
export const GrantRequest = defineSchema(
    schema.object({
        /** The principal acting for the delegator. */
        delegate: Subject,
        /** The permissions the delegate may use for the delegator. */
        permissions: schema.array(GrantPermission.schema),
        /** How often the delegator approves a call. */
        approval: GrantApproval,
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
    claim(grant: Grant): CallerGrant {
        return { id: grant.id, scope: grant.scope, permissions: [...grant.permissions] };
    },

    /** Read a grant by identifier as a snapshot shows it, absent once revoked. */
    async read(snapshot: Snapshot, id: Identifier<"grant">): Promise<Grant | undefined> {
        return (await snapshot.row(accessGrant, { id })) ?? undefined;
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
            !Subject.same(grant.delegator, parties.delegator) ||
            !Subject.same(grant.delegate, parties.delegate)
        ) {
            throw new AccessError("UNAUTHORIZED", `grant ${id} belongs to another delegate`);
        }

        // refuse a grant that needs approval for each call
        // TODO #Incomplete: phase E lands an "each" call on a one-call branch, answered APPROVAL_REQUIRED { branch }
        if (grant.approval === "each") {
            throw new AccessError("FORBIDDEN", `grant ${id} needs approval for each call`);
        }

        return grant;
    },
};
