import { none, principal } from "@destack/access";
import { unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { account } from "./account.ts";
import { user } from "./user.ts";

/** A set of principals roles bind to at once. */
export const group = defineObject({
    name: "group",
    tier: "global",
    plural: "groups",
    scope: account,
    fields: { name: field.string(schema.string().min(1).max(200)) },
    relations: {
        member: {
            subjects: [user, principal.installation, "group#member", account.members("member")],
        },
    },
    permissions: { create: none(), read: none(), update: none(), delete: none(), share: none() },
    shareable: { by: "share" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", { fields: ["name"] }),
        update: method.update("update", { fields: ["name"] }),
        delete: method.delete("delete"),
    },
    constraints: (group) => [
        unique("group_scope_name").on(group.scope, group.name),
        unique("group_scope_id").on(group.scope, group.id),
    ],
});
/** An account-local group. */
export type Group = Select<typeof group.table>;
