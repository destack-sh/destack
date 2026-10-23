import { Relationship } from "@destack/access";
import type { MethodContext } from "@destack/object";
import { type Group, group } from "../object/group.ts";
import { relationship } from "@destack/object";

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
        if (row.parentId === null) {
            return;
        }

        // relate the group's members to the parent as the store
        await context.mutate(context.self, [
            relationship.call("create", {
                ...relationship.in(row.scope),
                object: group.reference(row.scope, row.parentId),
                relation: "member",
                subject: { ...group.reference(row.scope, row.id), relation: "member" },
            }),
        ]);
    },

    /** Revoke the relationship of a group's parent to the group's members. */
    async release(context: MethodContext, row: Group): Promise<void> {
        // find the parent's relationship to the group's members
        if (row.parentId === null) {
            return;
        }
        const parent = group.reference(row.scope, row.parentId);
        const rows = await Relationship.readOn(context, parent, {
            relation: "member",
            subject: { ...group.reference(row.scope, row.id), relation: "member" },
        });

        // revoke each as the store
        await context.mutate(
            context.self,
            rows.map((related) =>
                relationship.call("delete", { ...relationship.in(related.scope), id: related.id }),
            ),
        );
    },
};
