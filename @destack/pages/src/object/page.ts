import { permission, relation, through, union, anyone } from "@destack/access";
import { account, group, user } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";

/** The people and sets pages are shared with. */
const MEMBERS = [user, group.members("member"), account.members("member")];

/** A page in a tree of pages, shared with its subpages and publishable through links. */
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: "space",
    parent: {
        object: "self",
        optional: true,
        receive: "edit",
        move: "manage",
    },
    fields: {
        owner: field.reference(user).caller(),
        title: field.string({ max: 500 }).default(""),
        icon: field.string({ max: 32 }).optional(),
        position: field.position().default("a0"),
    },
    deletion: { recovery: { days: 30 }, deletedBy: "manage" },
    relations: {
        editor: { subjects: MEMBERS },
        viewer: { subjects: [...MEMBERS, anyone.all()] },
    },
    permissions: {
        "read-direct": union(relation("owner"), relation("editor"), relation("viewer")),
        "edit-direct": union(relation("owner"), relation("editor")),
        "manage-direct": relation("owner"),
        read: union(permission("read-direct"), through("parent", "read-direct", true)),
        edit: union(permission("edit-direct"), through("parent", "edit-direct", true)),
        manage: union(permission("manage-direct"), through("parent", "manage-direct", true)),
    },
    grantedBy: "manage",
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    },
});
