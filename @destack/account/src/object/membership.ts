import { unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { identifier } from "@destack/schema";
import { user } from "./user.ts";

/** A user's own record of a space they joined, which lets the space's cell read their public profile. */
export const membership = defineObject({
    name: "membership",
    tier: "global",
    plural: "memberships",
    scope: user,
    fields: {
        /** The space the user joined. */
        spaceId: field.string(identifier("space")),
    },
    constraints: (entry) => [unique("membership_space").on(entry.scope, entry.spaceId)],
    permissions: ["read", "list", "create", "delete"],
    methods: {
        get: method.get("read"),
        list: method.list("list"),
        create: method.create("create", { fields: ["spaceId"] }),
        delete: method.delete("delete"),
    },
});
/** A user's record of a space they joined. */
export type Membership = Select<typeof membership.table>;
