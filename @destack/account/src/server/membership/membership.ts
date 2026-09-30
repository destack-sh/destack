import { accessRelationship, principal } from "@destack/access";
import { and, eq } from "@destack/db";
import type { Call } from "@destack/object";
import { SystemAuthorization } from "@destack/object/server";
import { Scope } from "@destack/sync";
import * as base from "../../object/membership.ts";
import { user } from "../../object/user.ts";

/** One call of a membership method. */
type MembershipCall = Call<typeof base.membership.table>;

/** Memberships that let the spaces a user joined read the user. */
export const membership = base.membership.handle({ create, delete: remove });

/** Record a joined space and let it read the user. */
async function create(call: MembershipCall, next: (call?: MembershipCall) => Promise<unknown>) {
    const created = await next();
    await (await joined(call)).grant(relationship(call, call.input.spaceId as string));

    return created;
}

/** Forget a joined space and stop it reading the user. */
async function remove(call: MembershipCall, next: (call?: MembershipCall) => Promise<unknown>) {
    // forget the space, then revoke the relationship joining it
    const removed = await next();
    const joining = relationship(call, call.target!.spaceId);
    const [row] = await call.database
        .select({ id: accessRelationship.id })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.packageId, joining.object.packageId),
                eq(accessRelationship.type, joining.object.type),
                eq(accessRelationship.objectId, joining.object.id),
                eq(accessRelationship.relation, joining.relation),
                eq(accessRelationship.subjectType, joining.subject.type),
                eq(accessRelationship.subjectId, joining.subject.id),
            ),
        );
    if (row === undefined) {
        throw new TypeError(
            `membership of ${call.scope} in ${joining.subject.id} lost its relationship`,
        );
    }
    await (await joined(call)).revoke(joining.object, row.id);

    return removed;
}

/** Open the system's authorization over the users in the call's database. */
function joined(call: MembershipCall): Promise<SystemAuthorization> {
    return SystemAuthorization.open(
        call.served().authorizer,
        call.database,
        Scope.universe.id,
        call.now,
    );
}

/** Relate the membership's user to the space it joined. */
function relationship(call: MembershipCall, spaceId: string) {
    return {
        object: user.reference(Scope.universe.id, call.scope),
        relation: "joined",
        subject: principal.space.reference(Scope.universe.id, spaceId),
    };
}
