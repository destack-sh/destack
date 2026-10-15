import type { CallOf, NextOf, ResultOf } from "@destack/object";
import { Member, member } from "../object/member.ts";

/** Implement members. */
export function implementMember(): typeof member {
    return member.implement({
        methods: {
            join,
            update,
            suspend: async (call, next) => {
                // keep another active owner beside a suspended owner
                await Member.requireOtherOwner(call, call.target);

                return transition(call, next);
            },
            reactivate: transition,
            delete: remove,
        },
    });
}

/** Seat a user as an active member, as the universe does for an accepted invitation. */
async function join(
    call: CallOf<typeof member, "join">,
    next: NextOf<typeof member, "join">,
): Promise<ResultOf<typeof member, "join">> {
    const row = await next();
    await Member.relate(call, row);

    return row;
}

/** Change a member's role or title, requiring ownership to make or unmake an owner. */
async function update(
    call: CallOf<typeof member, "update">,
    next: NextOf<typeof member, "update">,
): Promise<ResultOf<typeof member, "update">> {
    // require ownership around an owner's role and another owner beside a demoted one
    await Member.requireOwner(call, [call.target.role, call.input.role]);
    if (call.input.role !== undefined && call.input.role !== "owner") {
        await Member.requireOtherOwner(call, call.target);
    }

    // relate the user under the new role
    const row = await next();
    await Member.relate(call, row);

    return row;
}

/** Suspend or reactivate a member, requiring ownership for an owner. */
async function transition(
    call: CallOf<typeof member, "suspend" | "reactivate">,
    next: NextOf<typeof member, "suspend" | "reactivate">,
): Promise<Member> {
    // require ownership of an owner's seat
    await Member.requireOwner(call, [call.target.role]);

    // relate the user by the new status
    const row = await next();
    await Member.relate(call, row);

    return row;
}

/** Remove a member with its relationship, requiring ownership to remove an owner. */
async function remove(
    call: CallOf<typeof member, "delete">,
    next: NextOf<typeof member, "delete">,
): Promise<ResultOf<typeof member, "delete">> {
    // require ownership of an owner's seat and another owner
    await Member.requireOwner(call, [call.target.role]);
    await Member.requireOtherOwner(call, call.target);

    // release the seat's relationship
    await Member.release(call, call.target);

    return next();
}
