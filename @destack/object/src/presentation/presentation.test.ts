import { argument, Localization, plural, SOURCE_LOCALE, t } from "@destack/locale";
import { expect, test } from "@destack/test";
import { through } from "@destack/access";
import { now } from "@destack/db";
import { defineObject, field } from "../index.ts";
import { space } from "../test/space.ts";
import { ObjectAppearance } from "./appearance.ts";
import { ObjectDetail } from "./detail.ts";
import { ObjectView } from "./view.ts";

/** The source locale, which renders a message title as written. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

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

/** The moment views read their times at: noon of 2026-11-19 in Vienna. */
const MOMENT = { now: Date.UTC(2026, 10, 19, 12), timeZone: "Europe/Vienna" };

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
    color: "cyan",
    icon: null,
};

test("offer the declared views first, default to the first, and extend a query by a view", () => {
    // read the task's views, and extend a query by the declared one
    const [open] = task.views;
    const query = ObjectView.query(
        ObjectView.parse(open),
        { where: { projectId: "project-1" } },
        MOMENT,
    );

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
    const key = ObjectDetail.key(task, TASK, MOMENT, SOURCE, undefined);
    const full = ObjectDetail.full(project, PROJECT, [project, task], MOMENT, SOURCE, undefined);

    expect({
        appearance: ObjectAppearance.of(task, TASK, MOMENT, SOURCE, undefined),
        projectAppearance: ObjectAppearance.of(project, PROJECT, MOMENT, SOURCE, undefined),
        reference: key.reference,
        key: key.sections.map((section) => section.fields.map((each) => [each.field, each.value])),
        projectKey: ObjectDetail.key(
            project,
            PROJECT,
            MOMENT,
            SOURCE,
            undefined,
        ).sections[0]?.fields.map((each) => each.field),
        sections: ObjectDetail.full(task, TASK, [project, task], MOMENT, SOURCE, undefined)
            .sections,
        related: full.related,
    }).toEqual({
        appearance: { title: "Write docs", icon: undefined, color: "cyan" },
        projectAppearance: { title: "Launch", icon: { icon: "folder" }, color: undefined },
        reference: task.reference("space-1", "task-1"),
        key: [
            [
                ["state", { kind: "choice", value: "started" }],
                ["dueAt", { kind: "time", value: 1000 }],
            ],
        ],
        projectKey: ["lead"],
        sections: [
            {
                title: "Plan",
                fields: [
                    { field: "state", value: { kind: "choice", value: "started" } },
                    { field: "dueAt", value: { kind: "time", value: 1000 } },
                    {
                        field: "projectId",
                        value: { kind: "reference", value: "project-1", target: "project" },
                    },
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

test("color an object by the first condition it meets: overdue red, blocked yellow, else none", () => {
    // define chores colored by conditions rather than a stored color
    const chore = defineObject({
        name: "chore",
        plural: "chores",
        title: t`${plural(argument("count"), { one: "Chore", other: "Chores" })}`,
        scope: space,
        fields: {
            title: field.string(),
            state: field.enum(["todo", "blocked", "done"]),
            dueAt: field.time().optional(),
        },
        access: {
            permissions: { read: through("scope", "read"), write: through("scope", "read") },
        },
        presentable: {
            color: [
                {
                    where: { dueAt: { lt: now.start("day") }, state: { ne: "done" } },
                    swatch: "red",
                },
                { where: { state: "blocked" }, swatch: "yellow" },
            ],
        },
    });
    const yesterday = Date.UTC(2026, 10, 18, 9);
    const rows = [
        { id: "c1", scope: "space-1", title: "Pay", state: "blocked", dueAt: yesterday },
        { id: "c2", scope: "space-1", title: "Plan", state: "blocked", dueAt: null },
        { id: "c3", scope: "space-1", title: "Ship", state: "done", dueAt: yesterday },
    ];

    expect({
        colors: rows.map((row) => ObjectAppearance.of(chore, row, MOMENT, SOURCE, undefined).color),
        stored: Object.hasOwn(chore.fields, "color"),
    }).toEqual({ colors: ["red", "yellow", undefined], stored: false });
});

test("title every type by its title or name field, a message as rendered, else its number or identifier", () => {
    // declare a type titled by name, one titled by a message, and one with neither, none presentable
    const base = {
        scope: space,
        access: {
            permissions: { read: through("scope", "read"), write: through("scope", "read") },
        },
    };
    const named = defineObject({
        ...base,
        name: "folder",
        plural: "folders",
        title: t`Folders`,
        fields: { name: field.string() },
    });
    const labelled = defineObject({
        ...base,
        name: "plan",
        plural: "plans",
        title: t`Plans`,
        fields: { name: field.message() },
    });
    const bare = defineObject({
        ...base,
        name: "receipt",
        plural: "receipts",
        title: t`Receipts`,
        fields: { at: field.number() },
    });
    const counted = defineObject({
        ...base,
        name: "invoice",
        plural: "invoices",
        title: t`Invoices`,
        fields: { at: field.number() },
        numbered: { prefix: "INV-" },
    });
    const row = { id: "x1", scope: "space-1" };

    // read each title, and no decoration fields without the trait
    expect({
        named: ObjectAppearance.of(named, { ...row, name: "Archive" }, MOMENT, SOURCE, undefined),
        labelled: ObjectAppearance.of(labelled, { ...row, name: t`Pro` }, MOMENT, SOURCE, undefined)
            .title,
        bare: ObjectAppearance.of(bare, { ...row, at: 1 }, MOMENT, SOURCE, undefined).title,
        counted: ObjectAppearance.title(counted, { ...row, at: 1, number: 7 }, SOURCE, undefined),
        fields: [named, labelled, bare].map((type) => type.presentation.title),
        decorated: Object.hasOwn(named.fields, "icon") || Object.hasOwn(named.fields, "color"),
    }).toEqual({
        named: { title: "Archive", icon: undefined, color: undefined },
        labelled: "Pro",
        bare: "x1",
        counted: "INV-7",
        fields: ["name", "name", undefined],
        decorated: false,
    });
});
