import { argument, plural, t } from "@destack/locale";
import { expect, test } from "@destack/test";
import { through } from "@destack/access";
import { defineObject, field } from "../index.ts";
import { space } from "../test/space.ts";
import { ObjectAppearance } from "./appearance.ts";
import { ObjectDetail } from "./detail.ts";
import { ObjectView } from "./view.ts";

/** Projects with a fixed icon, presented with every default. */
const project = defineObject({
    name: "project",
    plural: "projects",
    title: t`${plural(argument("count"), { one: "Project", other: "Projects" })}`,
    scope: space,
    fields: { name: field.string(), lead: field.string().optional() },
    access: { permissions: { read: through("scope", "read"), write: through("scope", "read") } },
    presentable: { icon: "folder", color: false },
});

/** Tasks in a project, with a state, a due time, a declared card and page, and a declared view. */
const task = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
    scope: space,
    fields: {
        title: field.string(),
        state: field.enum(["todo", "started", "done"]),
        project: field.reference(project),
        dueAt: field.time().optional(),
        notes: field.text().optional(),
    },
    access: { permissions: { read: through("scope", "read"), write: through("scope", "read") } },
    presentable: {
        card: ["state", "dueAt"],
        page: { sections: [{ title: "Plan", fields: ["state", "dueAt", "projectId"] }] },
        views: [
            {
                name: "open",
                title: t`Open`,
                where: { state: { ne: "done" } },
                layout: { kind: "board", by: "state", empty: "hide" },
            },
        ],
    },
});

/** A space's project, as a row. */
const PROJECT = { id: "project-1", scope: "space-1", name: "Launch", lead: null, icon: null };

/** A task in the project, as a row. */
const TASK = {
    id: "task-1",
    scope: "space-1",
    title: "Write docs",
    state: "started",
    projectId: "project-1",
    dueAt: 1000,
    notes: "a long note",
    color: "teal",
    icon: null,
};

test("offer the declared views first, default to the first, and extend a query by a view", () => {
    // read the task's views, and extend a query by the declared one
    const [open] = task.views;
    const query = ObjectView.query(ObjectView.parse(open), { where: { projectId: "project-1" } });

    expect({
        views: task.views.map((view) => view.name),
        defaults: [task.view.name, project.view.name],
        query,
    }).toEqual({
        views: ["open", "table", "board-by-state", "calendar-by-due-at"],
        defaults: ["open", "table"],
        query: {
            where: { AND: [{ projectId: "project-1" }, { state: { ne: "done" } }] },
            orderBy: { state: "asc" },
        },
    });
});

test("read an object's appearance, its key fields by the declared card or the first shown ones, and its full detail", () => {
    const key = ObjectDetail.key(task, TASK);
    const full = ObjectDetail.full(project, PROJECT, [project, task]);

    expect({
        appearance: ObjectAppearance.of(task, TASK),
        projectAppearance: ObjectAppearance.of(project, PROJECT),
        reference: key.reference,
        key: key.sections.map((section) => section.fields.map((each) => [each.field, each.value])),
        projectKey: ObjectDetail.key(project, PROJECT).sections[0]?.fields.map(
            (each) => each.field,
        ),
        sections: ObjectDetail.full(task, TASK, [project, task]).sections,
        related: full.related,
    }).toEqual({
        appearance: { title: "Write docs", icon: undefined, color: "teal" },
        projectAppearance: { title: "Launch", icon: { icon: "folder" }, color: undefined },
        reference: task.reference("space-1", "task-1"),
        key: [
            [
                ["state", "started"],
                ["dueAt", 1000],
            ],
        ],
        projectKey: ["lead"],
        sections: [
            {
                title: "Plan",
                fields: [
                    { field: "state", value: "started", target: undefined },
                    { field: "dueAt", value: 1000, target: undefined },
                    { field: "projectId", value: "project-1", target: "project" },
                ],
            },
        ],
        related: [
            {
                type: "task",
                field: "projectId",
                view: {
                    name: "task-by-project-id",
                    title: task.definition.title,
                    where: { projectId: "project-1" },
                    layout: { kind: "table" },
                },
            },
        ],
    });
});
