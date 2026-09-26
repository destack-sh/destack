import { user } from "@destack/account/object";
import { relation, through, union } from "@destack/access";
import { defineObject, field, method } from "@destack/object";
import { project } from "./project.ts";

/** A piece of work in a project, done by its assignee. */
export const task = defineObject({
    name: "task",
    plural: "tasks",
    scope: "space",
    parent: { object: project, delete: "cascade", receive: "plan" },
    fields: {
        author: field.reference(user).caller(),
        title: field.string({ min: 1, max: 500 }),
        assignee: field.reference(user).optional(),
        due: field.time().optional(),
        priority: field.enum(["low", "normal", "high", "urgent"]).default("normal"),
        budget: field.number().optional().guard({ read: "manage", write: "manage" }),
        origin: field.reference("self", { qualified: true }).optional(),
    },
    states: {
        initial: "open",
        transitions: {
            start: { from: ["open"], to: "active", permission: "work" },
            complete: { from: ["open", "active"], to: "done", permission: "work" },
            cancel: { from: ["open", "active"], to: "cancelled", permission: "plan" },
            reopen: { from: ["done", "cancelled"], to: "open", permission: "work" },
        },
    },
    permissions: {
        read: union(relation("assignee"), through("parent", "read")),
        work: union(relation("assignee"), through("parent", "plan")),
        plan: through("parent", "plan"),
        manage: through("parent", "manage"),
    },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("plan"),
        update: method.update("plan"),
        delete: method.delete("plan"),
    },
});
