import { argument, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";

/** Where a task stands. */
export const taskStatus = field.enum({
    review: { title: t`In review`, icon: "circle-half-tilt", color: "green" },
    doing: { title: t`In progress`, icon: "circle-half", color: "yellow" },
    todo: { title: t`To do`, icon: "circle", color: "gray" },
    backlog: { title: t`Backlog`, icon: "circle-dashed", color: "gray" },
    done: { title: t`Done`, icon: "check-circle", color: "blue" },
    canceled: { title: t`Canceled`, icon: "x-circle", color: "gray" },
});

/** How pressing a task is. */
export const taskPriority = field.enum({
    urgent: { title: t`Urgent`, icon: "warning", color: "orange" },
    high: { title: t`High`, icon: "cell-signal-full" },
    medium: { title: t`Medium`, icon: "cell-signal-medium" },
    low: { title: t`Low`, icon: "cell-signal-low" },
    none: { title: t`No priority`, icon: "dots-three" },
});

/** The studio, which has tasks. */
const studio = defineObject({
    name: "studio",
    plural: "studios",
    title: t`${plural(argument("count"), { one: "Studio", other: "Studios" })}`,
    scope: Scope.universe.id,
    fields: {},
    container: "scope",
});

/** A piece of the studio's work. */
export const taskType = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "task", other: "tasks" })}`,
    scope: studio,
    fields: {
        /** The title. */
        title: field.string(schema.string()),
        /** Where it stands. */
        status: taskStatus,
        /** How pressing it is. */
        priority: taskPriority,
        /** Who does it. */
        assignee: field.string(schema.string()).optional(),
        /** The project it belongs to. */
        project: field.string(schema.string()).optional(),
    },
});

/** What a person wrote about a task. */
export const commentType = defineObject({
    name: "comment",
    plural: "comments",
    title: t`${plural(argument("count"), { one: "comment", other: "comments" })}`,
    scope: studio,
    fields: {
        /** The words. */
        text: field.string(schema.string()),
    },
});

/** The studio the tasks are in. */
export const STUDIO = "studio-01996ab0-0000-7000-8000-000000000001";
