import type { MethodContextOf, NextOf, ResultOf } from "@destack/object";
import { Member, member } from "../object/member.ts";

/** Implement members. */
export function implementMember(): typeof member {
    return member.implement({
        methods: {
            join,
            update,
            suspend: async (call, next) => {
                // require another active owner beside a suspended owner
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
    context: MethodContextOf<typeof member, "join">,
    next: NextOf<typeof member, "join">,
): Promise<ResultOf<typeof member, "join">> {
    const row = await next();
    await Member.relate(context, row);

    return row;
}

/** Change a member's role or title, requiring ownership to make or unmake an owner. */
async function update(
    context: MethodContextOf<typeof member, "update">,
    next: NextOf<typeof member, "update">,
): Promise<ResultOf<typeof member, "update">> {
    // require ownership around an owner's role and another owner beside a demoted one
    await Member.requireOwner(context, [context.target.role, context.input.role]);
    if (context.input.role !== undefined && context.input.role !== "owner") {
        await Member.requireOtherOwner(context, context.target);
    }

    // relate the user under the new role
    const row = await next();
    await Member.relate(context, row);

    return row;
}

/** Suspend or reactivate a member, requiring ownership for an owner. */
async function transition(
    context: MethodContextOf<typeof member, "suspend" | "reactivate">,
    next: NextOf<typeof member, "suspend" | "reactivate">,
): Promise<Member> {
    // require ownership of an owner's seat
    await Member.requireOwner(context, [context.target.role]);

    // relate the user by the new status
    const row = await next();
    await Member.relate(context, row);

    return row;
}

/** Remove a member with its relationship, requiring ownership to remove an owner. */
async function remove(
    context: MethodContextOf<typeof member, "delete">,
    next: NextOf<typeof member, "delete">,
): Promise<ResultOf<typeof member, "delete">> {
    // require ownership of an owner's seat and another owner
    await Member.requireOwner(context, [context.target.role]);
    await Member.requireOtherOwner(context, context.target);

    // release the seat's relationship
    await Member.release(context, context.target);

    return next();
}
