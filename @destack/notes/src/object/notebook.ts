import { relation, union } from "@destack/access";
import { account, group, user } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";

/** The people and sets a notebook or note is shared with. */
export const SHARED_WITH = [user, group.members("member"), account.members("member")];

/** A collection of notes, shared with everyone who works on them. */
export const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    scope: "space",
    fields: {
        owner: field.reference(user).caller(),
        name: field.string({ min: 1, max: 200 }),
        noteCount: field.count(),
    },
    relations: {
        editor: { subjects: SHARED_WITH },
        viewer: { subjects: SHARED_WITH },
    },
    permissions: {
        read: union(relation("owner"), relation("editor"), relation("viewer")),
        edit: union(relation("owner"), relation("editor")),
        manage: relation("owner"),
    },
    grantedBy: "manage",
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    },
});
