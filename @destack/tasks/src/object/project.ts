import { principal, relation, union } from "@destack/access";
import { account, group } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** The people and sets projects are shared with. */
const MEMBERS = [principal.user, group.members("member"), account.members("member")];

/** A body of work in a space, planned by its members and followed by its viewers. */
export const project = defineObject({
    name: "project",
    plural: "projects",
    scope: space,
    fields: {
        /** The user who created the project. */
        owner: field.reference(principal.user).caller(),
        /** The project name. */
        name: field.string(schema.string().min(1).max(200)),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    relations: {
        member: { subjects: MEMBERS },
        viewer: { subjects: MEMBERS },
    },
    permissions: {
        read: union(relation("owner"), relation("member"), relation("viewer")),
        plan: union(relation("owner"), relation("member")),
        manage: relation("owner"),
    },
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
    },
});
