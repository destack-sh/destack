import type { MethodContext } from "@destack/object";
import { type Group, group } from "../object/group.ts";
import { type RelatedFields, StoreRelationship } from "./relationship.ts";

/** Implement groups. */
export function implementGroup(): typeof group {
    return group.implement({
        methods: {
            create: async (call, next) => {
                // relate the new group's members to its parent
                const created = await next();
                await Nesting.relate(call, created);

                return created;
            },
            move: async (call, next) => {
                // relate the moved group's members to its new parent alone
                await Nesting.release(call, call.target);
                const moved = await next();
                await Nesting.relate(call, moved);

                return moved;
            },
            delete: async (call, next) => {
                // release the parent's relationship to the removed group's members
                await Nesting.release(call, call.target);

                return next();
            },
        },
    });
}

/** The relationship of a parent group to the members of a group nested in it. */
const Nesting = {
    /** Make a group's members members of its parent, if it has one. */
    async relate(context: MethodContext, row: Group): Promise<void> {
        const nesting = Nesting.of(row);
        if (nesting !== undefined) {
            await StoreRelationship.touch(context, nesting);
        }
    },

    /** Revoke the relationship of a group's parent to the group's members, if it has a parent. */
    async release(context: MethodContext, row: Group): Promise<void> {
        const nesting = Nesting.of(row);
        if (nesting !== undefined) {
            await StoreRelationship.delete(context, nesting);
        }
    },

    /** Read the relationship of a group's parent to its members, absent for a top-level group. */
    of(row: Group): RelatedFields | undefined {
        return row.parentId === null
            ? undefined
            : {
                  object: group.reference(row.scope, row.parentId),
                  relation: "member",
                  subject: { ...group.reference(row.scope, row.id), relation: "member" },
              };
    },
};
