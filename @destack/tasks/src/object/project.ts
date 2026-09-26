import { relation, union } from "@destack/access";
import { account, group, user } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";

/** A body of work in a space, planned by its members and followed by its viewers. */
export const project = defineObject({
    name: "project",
    plural: "projects",
    scope: "space",
    fields: {
        owner: field.reference(user).caller(),
        name: field.string({ min: 1, max: 200 }),
    },
    deletion: { recovery: { days: 30 }, deletedBy: "manage" },
    relations: {
        member: {
            subjects: [user, group.members("member"), account.members("member")],
        },
        viewer: {
            subjects: [user, group.members("member"), account.members("member")],
        },
    },
    permissions: {
        read: union(relation("owner"), relation("member"), relation("viewer")),
        plan: union(relation("owner"), relation("member")),
        manage: relation("owner"),
    },
    grantedBy: "manage",
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
    },
});
