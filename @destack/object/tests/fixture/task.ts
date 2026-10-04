import { schema } from "@destack/schema";
import { journal } from "@destack/audit";
import { relation, through, union, principal } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { defineService } from "@destack/service";
import { defineObject, field } from "../../src/index.ts";
import { user } from "../schema.ts";
import { space } from "./space.ts";

/** A body of work in a space, planned by its members and followed by its viewers. */
export const project = defineObject({
    name: "project",
    plural: "projects",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        name: field.string(schema.string().min(1).max(200)),
    },
    recoverable: { within: { days: 30 }, by: "manage" },
    relations: {
        member: {
            subjects: [user],
        },
        viewer: {
            subjects: [user],
        },
    },
    permissions: {
        read: union(relation("owner"), relation("member"), relation("viewer")),
        plan: union(relation("owner"), relation("member")),
        manage: relation("owner"),
    },
    shareable: { by: "manage" },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("manage"),
        update: method.update("manage"),
    }),
});

/** A piece of work in a project, done by its assignee. */
export const task = defineObject({
    name: "task",
    plural: "tasks",
    scope: space,
    nested: { in: project, delete: "cascade", receive: "plan" },
    fields: {
        author: field.reference(principal.user).caller(),
        title: field.string(schema.string().min(1).max(500)),
        assignee: field.reference(principal.user).optional(),
        due: field.time().optional(),
        priority: field.enum(["low", "normal", "high", "urgent"]).default("normal"),
        budget: field.number().optional().guard({ read: "manage", write: "manage" }),
        origin: field.reference("self", { qualified: true }).optional(),
        status: field.state({
            initial: "open",
            transitions: {
                start: { from: ["open"], to: "active", permission: "work" },
                complete: { from: ["open", "active"], to: "done", permission: "work" },
                cancel: { from: ["open", "active"], to: "cancelled", permission: "plan" },
                reopen: { from: ["done", "cancelled"], to: "open", permission: "work" },
            },
        }),
    },
    permissions: {
        read: union(relation("assignee"), through("parent", "read")),
        work: union(relation("assignee"), through("parent", "plan")),
        plan: through("parent", "plan"),
        manage: through("parent", "manage"),
    },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("plan"),
        update: method.update("plan"),
        delete: method.delete("plan"),
    }),
});

/** A remark on a task, made and read by the task's readers. */
export const comment = defineObject({
    name: "comment",
    plural: "comments",
    scope: space,
    nested: { in: task, delete: "cascade", receive: "read" },
    fields: {
        author: field.reference(principal.user).caller(),
        text: field.string(schema.string().min(1).max(10_000)),
    },
    permissions: { read: through("parent", "read"), edit: relation("author") },
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("read"),
        update: method.update("edit"),
        delete: method.delete("edit"),
    }),
});

/** Projects, tasks and comments, with their sharing and sync. */
export const tasksService = defineService("tasks", { objects: { project, task, comment } });

/** The database of one space's projects, tasks and comments. */
export const tasksDatabase = defineDatabase({
    name: "main",
    tables: [...project.tables, ...task.tables, ...comment.tables, journal],
});
