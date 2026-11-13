import { testIdentity } from "../../src/test/index.ts";
import { schema } from "@destack/schema";
import { relation, through, union, principal } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { definePackage } from "@destack/object";
import { defineObject, field } from "../../src/index.ts";
import { user } from "../schema.ts";
import { space } from "./space.ts";
import { tablesOf } from "../../src/index.ts";
import objectPackage from "../../src/package.ts";
import { invitation } from "../../src/access/index.ts";
import { nested, recoverable, shareable } from "../../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** A body of work in a space, planned by its members and followed by its viewers. */
export const project = defineObject(
    {
        name: "project",
        plural: "projects",
        title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            name: field.string(schema.string().min(1).max(200)),
        },
        access: {
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
        },
        traits: [recoverable({ within: { days: 30 }, by: "manage" }), shareable({ by: "manage" })],
    },
    (method) => ({
        create: method.create("manage"),
        update: method.update("manage"),
    }),
);

/** A piece of work in a project, done by its assignee. */
export const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: space,
        fields: {
            author: field.reference(principal.user).caller(),
            title: field.string(schema.string().min(1).max(500)),
            assignee: field.reference(principal.user).optional(),
            due: field.time().optional(),
            priority: field.enum(["low", "normal", "high", "urgent"]).default("normal"),
            budget: field.number().optional().guard({ read: "manage", write: "manage" }),
            origin: field.object().optional(),
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
        access: {
            permissions: {
                read: union(relation("assignee"), through("parent", "read")),
                work: union(relation("assignee"), through("parent", "plan")),
                plan: through("parent", "plan"),
                manage: through("parent", "manage"),
            },
        },
        traits: [nested({ in: project, delete: "cascade", add: "plan" })],
    },
    (method) => ({
        create: method.create("plan"),
        update: method.update("plan"),
        delete: method.delete("plan"),
    }),
);

/** A remark on a task, made and read by the task's readers. */
export const comment = defineObject(
    {
        name: "comment",
        plural: "comments",
        title: t`${plural(argument("count"), { one: "Comment", other: "Comments" })}`,
        scope: space,
        fields: {
            author: field.reference(principal.user).caller(),
            text: field.string(schema.string().min(1).max(10_000)),
        },
        access: {
            permissions: { read: through("parent", "read"), edit: relation("author") },
        },
        traits: [nested({ in: task, delete: "cascade", add: "read" })],
    },
    (method) => ({
        create: method.create("read"),
        update: method.update("edit"),
        delete: method.delete("edit"),
    }),
);

/** Projects, tasks and comments, with their sharing and sync. */
export const tasksPackage = definePackage({
    ...testIdentity,
    objects: [project, task, comment, invitation],
});

/** The database of one space's projects, tasks and comments. */
export const tasksDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { project, task, comment }, tables: [] }],
        follows: [],
        tables: [],
    }),
});
