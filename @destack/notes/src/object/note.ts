import { principal, relation, through, union } from "@destack/access";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { space } from "@destack/space/object";
import { notebook, SHARED_WITH } from "./notebook.ts";

/** A note, private to its owner until shared directly or through its notebook. */
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    nested: {
        in: notebook,
        delete: "cascade",
        optional: true,
        receive: "edit",
        move: "manage",
    },
    fields: {
        /** The user who wrote the note. */
        owner: field.reference(principal.user).caller(),
        /** The note title. */
        title: field.string(schema.string().max(500)).default(""),
        /** The note text, which collaborators edit together. */
        body: field.text(),
        /** Whether the note stays at the top of its notebook. */
        pinned: field.boolean().default(false),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    aggregates: { noteCount: { function: "count", where: { deletionRequestedAt: null } } },
    relations: {
        editor: { subjects: SHARED_WITH },
        viewer: { subjects: SHARED_WITH },
    },
    permissions: {
        read: union(
            relation("owner"),
            relation("editor"),
            relation("viewer"),
            through("parent", "read"),
        ),
        edit: union(relation("owner"), relation("editor"), through("parent", "edit")),
        manage: union(relation("owner"), through("parent", "manage")),
    },
    shareable: { by: "manage" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit", { fields: ["title", "pinned"] }),
        update: method.update("edit", { fields: ["title", "pinned"] }),
    },
});
