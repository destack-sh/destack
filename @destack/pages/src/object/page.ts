import { anyone, permission, principal, relation, through, union } from "@destack/access";
import { account, group } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** The people and sets pages are shared with. */
const MEMBERS = [principal.user, group.members("member"), account.members("member")];

/** A page in a tree of pages, shared with its subpages and publishable through links. */
export const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    nested: {
        in: "self",
        optional: true,
        receive: "edit",
        move: "manage",
    },
    fields: {
        /** The user who created the page. */
        owner: field.reference(principal.user).caller(),
        /** The page title. */
        title: field.string(schema.string().max(500)).default(""),
        /** The emoji or icon name shown beside the title. */
        icon: field.string(schema.string().max(32)).optional(),
        /** The page's place among its siblings, as a fractional index. */
        position: field.position().default("a0"),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
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
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    },
});
