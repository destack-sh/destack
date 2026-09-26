import { user } from "@destack/account/object";
import { relation, through } from "@destack/access";
import { defineObject, field, method } from "@destack/object";
import { task } from "./task.ts";

/** A remark on a task, which everyone who reads the task may make and read. */
export const comment = defineObject({
    name: "comment",
    plural: "comments",
    scope: "space",
    parent: { object: task, delete: "cascade", receive: "read" },
    fields: {
        author: field.reference(user).caller(),
        text: field.string({ min: 1, max: 10_000 }),
    },
    permissions: { read: through("parent", "read"), edit: relation("author") },
    methods: {
        list: method.list("read"),
        create: method.create("read"),
        update: method.update("edit"),
        delete: method.delete("edit"),
    },
});
