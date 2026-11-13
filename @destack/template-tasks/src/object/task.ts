import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import {
    activity,
    defineObject,
    field,
    nested,
    orderable,
    presentable,
    recoverable,
    searchable,
    tracked,
} from "@destack/object";
import { schema } from "@destack/schema";
import { cycle } from "./cycle.ts";
import { label } from "./label.ts";
import { project } from "./project.ts";

/** Where a task stands. */
export const TASK_STATES = ["backlog", "todo", "started", "done", "canceled"] as const;

/** How urgent a task is: none, then urgent to low, in this order. */
export const PRIORITIES = ["none", "urgent", "high", "medium", "low"] as const;

/** The highest priority number, low's. */
const LOWEST_PRIORITY = PRIORITIES.length - 1;

/** A piece of work in a space, tracked from backlog to done. */
export const task = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The task's title. */
        title: field.string(schema.string().min(1).max(300)),
        /** What the task is about. */
        description: field.string(schema.string().max(20_000)).default(""),
        /** Where the task stands. */
        state: field.enum(TASK_STATES).default("backlog"),
        /** How urgent the task is: 0 none, then 1 urgent to 4 low. */
        priority: field.integer().default(0),
        /** The person working on the task. */
        assignee: field.subject().optional(),
        /** The project the task belongs to. */
        project: field.reference(project, { delete: "keep" }).optional(),
        /** The cycle the task is planned into. */
        cycle: field.reference(cycle, { delete: "keep" }).optional(),
        /** The task's size in points. */
        estimate: field.integer().optional(),
        /** The moment the task is due by. */
        dueAt: field.time().optional(),
    },
    constraints: {
        priority: { kind: "check", condition: { priority: { gte: 0, lte: LOWEST_PRIORITY } } },
        estimate: {
            kind: "check",
            condition: { OR: [{ estimate: { isNull: true } }, { estimate: { gte: 0 } }] },
        },
    },
    traits: [
        presentable({ subtitle: "description" }),
        searchable({ text: ["title", "description"], sort: ["priority", "dueAt"] }),
        orderable({ within: "state" }),
        nested({ in: "self", optional: true, delete: "trash", add: "write" }),
        tracked({ by: "write", activity }),
        recoverable({ within: { days: 30 }, by: "write" }),
    ],
});

/** A label on a task. */
export const taskLabel = defineObject({
    name: "task-label",
    plural: "taskLabels",
    title: t`${plural(argument("count"), { one: "Task label", other: "Task labels" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The labeled task. */
        task: field.reference(task, { delete: "cascade" }),
        /** The label. */
        label: field.reference(label, { delete: "cascade" }),
    },
});

/** How one task relates to another. */
export const RELATION_KINDS = ["blocks", "duplicates", "relates"] as const;

/** A relation from one task to another: it blocks it, duplicates it, or relates to it. */
export const taskRelation = defineObject({
    name: "task-relation",
    plural: "taskRelations",
    title: t`${plural(argument("count"), { one: "Task relation", other: "Task relations" })}`,
    scope: space,
    access: {
        permissions: { read: through("scope", "read"), write: through("scope", "write") },
    },
    fields: {
        /** The task the relation is from. */
        task: field.reference(task, { delete: "cascade" }),
        /** The task the relation is to. */
        target: field.reference(task, { delete: "cascade" }),
        /** How the task relates to the target. */
        kind: field.enum(RELATION_KINDS),
    },
    constraints: {
        pair: { kind: "index", on: ["taskId", "targetId", "kind"], unique: true },
    },
});
