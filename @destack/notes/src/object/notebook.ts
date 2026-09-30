import { principal, relation, union } from "@destack/access";
import { account, group } from "@destack/account/object";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";

/** The people and sets a notebook or note is shared with. */
export const SHARED_WITH = [principal.user, group.members("member"), account.members("member")];

/** A collection of notes, shared with everyone who works on them. */
export const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    scope: space,
    fields: {
        /** The user who created the notebook. */
        owner: field.reference(principal.user).caller(),
        /** The notebook name. */
        name: field.string(schema.string().min(1).max(200)),
        /** How many notes outside the trash the notebook holds. */
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
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
        delete: method.delete("manage"),
    },
});
