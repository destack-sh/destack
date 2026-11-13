import { through } from "@destack/access";
import { space } from "@destack/account/object";
import { argument, plural, t } from "@destack/locale";
import {
    defineObject,
    field,
    orderable,
    presentable,
    recoverable,
    searchable,
} from "@destack/object";
import { schema } from "@destack/schema";
import { cycle } from "./cycle.ts";
import { label } from "./label.ts";
import { project } from "./project.ts";

/** Where a task stands, as Linear's workflow state types. */
export const TASK_STATES = ["backlog", "todo", "started", "done", "canceled"] as const;

/** How urgent a task is, as Linear's priorities. */
export const PRIORITIES = ["none", "urgent", "high", "medium", "low"] as const;

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
        /** How urgent the task is. */
        priority: field.enum(PRIORITIES).default("none"),
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
    traits: [
        presentable({ subtitle: "description" }),
        searchable({ text: ["title", "description"], sort: ["priority", "dueAt"] }),
        orderable({ within: "state" }),
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
