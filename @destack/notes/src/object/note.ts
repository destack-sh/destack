import { user } from "@destack/account/object";
import { relation, through, union } from "@destack/access";
import { defineObject, field, method } from "@destack/object";
import { notebook, SHARED_WITH } from "./notebook.ts";

/** A plain-text note, private to its owner until shared directly or through its notebook. */
export const note = defineObject({
    name: "note",
    plural: "notes",
    scope: "space",
    parent: {
        object: notebook,
        delete: "cascade",
        optional: true,
        receive: "edit",
        move: "manage",
    },
    fields: {
        owner: field.reference(user).caller(),
        title: field.string({ max: 500 }).default(""),
        text: field.string({ max: 100_000 }).default(""),
        pinned: field.boolean().default(false),
    },
    deletion: { recovery: { days: 30 }, deletedBy: "manage" },
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
    grantedBy: "manage",
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    },
});
