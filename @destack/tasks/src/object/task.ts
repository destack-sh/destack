import { principal, relation, through, union } from "@destack/access";
import { announcement, notification, subscription } from "@destack/notification";
import { defineObject, field, method } from "@destack/object";
import { schema } from "@destack/schema";
import { comment } from "@destack/social";
import { space } from "@destack/space/object";
import { project } from "./project.ts";

/** A piece of work in a project, done by its assignee. */
export const task = defineObject({
    name: "task",
    plural: "tasks",
    scope: space,
    nested: { in: project, delete: "cascade", receive: "plan" },
    fields: {
        /** The user who created the task. */
        author: field.reference(principal.user).caller(),
        /** The task title. */
        title: field.string(schema.string().min(1).max(500)),
        /** The user working on the task. */
        assignee: field.reference(principal.user).optional(),
        /** When the task is due, in UTC epoch milliseconds. */
        due: field.time().optional(),
        /** How urgent the task is. */
        priority: field.enum(["low", "normal", "high", "urgent"]).default("normal"),
        /** The task budget, for project managers only. */
        budget: field.number().optional().guard({ read: "manage", write: "manage" }),
        /** The task this one came from, possibly in another space. */
        origin: field.reference("self", { qualified: true }).optional(),
        /** The number of comments on the task. */
        commentCount: field.count(),
        /** Where the task stands in its workflow. */
        status: field.state({
            initial: "open",
            transitions: {
                start: { from: ["open"], to: "active", permission: "work" },
                complete: { from: ["open", "active"], to: "done", permission: "work" },
                cancel: { from: ["open", "active"], to: "cancelled", permission: "edit" },
                reopen: { from: ["done", "cancelled"], to: "open", permission: "work" },
            },
        }),
    },
    permissions: {
        read: union(relation("assignee"), through("parent", "read")),
        work: union(relation("assignee"), through("parent", "plan")),
        edit: through("parent", "plan"),
        manage: through("parent", "manage"),
    },
    attachments: [
        comment.attach({ by: "read" }),
        notification.attach({ by: "read" }),
        announcement.attach({ by: "read" }),
        subscription.attach({ by: "read" }),
    ],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
        delete: method.delete("edit"),
    },
});
