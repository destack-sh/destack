import { none, principal, relation, Relationship, through, union } from "@destack/access";
import { type Select } from "@destack/db";
import { type MethodContext, defineObject, field } from "@destack/object";
import { CallError } from "@destack/runtime/error";
import { Scope } from "@destack/sync";
import { schema } from "@destack/schema";
import { DEFAULT_MEMBER_ROLE, MEMBER_ROLES, organisation } from "./organisation.ts";
import { relationship } from "./relationship.ts";

/** The longest member title, in characters: a long job title with room to spare. */
const TITLE_LENGTH = 200;

/** A user's membership in an organisation, from an accepted invitation or the universe. */
export const member = defineObject(
    {
        name: "member",
        plural: "members",
        scope: organisation,
        fields: {
            /** The member user. */
            user: field.reference(principal.user),
            /** The role the member has while active. */
            role: field.enum(MEMBER_ROLES).default(DEFAULT_MEMBER_ROLE),
            /** The member's job title within the organisation. */
            title: field.string(schema.string().min(1).max(TITLE_LENGTH)).optional(),
            /** Whether the membership is active or suspended. */
            status: field.state({
                initial: "active",
                transitions: {
                    suspend: { from: ["active"], to: "suspended", permission: "manage" },
                    reactivate: { from: ["suspended"], to: "active", permission: "manage" },
                },
            }),
        },
        permissions: {
            read: union(relation("user"), through("organisation", "read")),
            manage: through("organisation", "manage"),
            join: none(),
        },
        constraints: {
            user: { kind: "unique", on: ["userId", "scope"] },
        },
    },
    (method) => ({
        join: method.create("join", { isInternal: true, fields: ["userId", "role", "title"] }),
        update: method.update("manage", { fields: ["role", "title"] }),
        delete: method.delete("manage"),
    }),
);
/** A persisted member. */
export type Member = Select<typeof member.table>;

/** Members, stored as relationships of their organisations to their users. */
export const Member = {
    /** Require ownership of the call's organisation when any of some roles is an owner's. */
    async requireOwner(
        context: MethodContext,
        roles: readonly (string | undefined)[],
    ): Promise<void> {
        // let administrators change roles below owner
        if (!roles.includes("owner")) {
            return;
        }

        // require the organisation's ownership
        const target = organisation.reference(Scope.universe.id, context.scope);
        const decision = await context.check(organisation.permission("own"), target);
        if (!decision.isAllowed) {
            throw new CallError("FORBIDDEN", {
                message: `only owners of ${context.scope} change its owners`,
            });
        }
    },

    /** Refuse losing an organisation's last active owner when a change takes an owner's seat. */
    async requireOtherOwner(context: MethodContext, row: Member): Promise<void> {
        // let changes of other seats pass
        if (row.role !== "owner" || row.status !== "active") {
            return;
        }

        // require another active owner
        const others = await context.read(member).findMany({
            where: {
                scope: row.scope,
                role: "owner",
                status: "active",
                id: { ne: row.id },
            },
        });
        if (others.length === 0) {
            throw new CallError("PRECONDITION_FAILED", {
                message: `${row.scope} needs at least one owner`,
            });
        }
    },

    /** Relate an active member's user to its organisation under its role. */
    async relate(context: MethodContext, row: Member): Promise<void> {
        // revoke what the member related before
        await Member.release(context, row);

        // relate the user of an active member under its role, as the store
        if (row.status === "active") {
            await context.mutate(context.self, [
                relationship.call("create", {
                    ...relationship.in(Scope.universe.id),
                    object: organisation.reference(Scope.universe.id, row.scope),
                    relation: row.role,
                    subject: principal.user.reference(Scope.universe.id, row.userId),
                }),
            ]);
        }
    },

    /** Revoke every relationship of a member's organisation to the member's user. */
    async release(context: MethodContext, row: Pick<Member, "scope" | "userId">): Promise<void> {
        // revoke them as the store
        const object = organisation.reference(Scope.universe.id, row.scope);
        await context.mutate(
            context.self,
            (await relationships(context, row)).map((relationshipId) =>
                relationship.call("delete", {
                    ...relationship.in(Scope.universe.id),
                    object,
                    relationshipId,
                }),
            ),
        );
    },
};

/** List the relationships of a member's organisation to the member's user. */
async function relationships(
    context: MethodContext,
    row: Pick<Member, "scope" | "userId">,
): Promise<string[]> {
    const object = organisation.reference(Scope.universe.id, row.scope);
    const rows = await Relationship.readOn(context, object, {
        subject: principal.user.reference(Scope.universe.id, row.userId),
    });

    return rows.map((entry) => entry.id);
}
