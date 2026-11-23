import { argument, Localization, plural, SOURCE_LOCALE, t } from "@destack/locale";
import { expect, test } from "@destack/test";
import { through } from "@destack/access";
import { defineObject, field } from "../index.ts";
import { nested } from "../trait/index.ts";
import { space } from "../test/space.ts";
import { Layout } from "./layout.ts";
import { BoardLayout } from "./board.ts";
import { CalendarLayout } from "./calendar.ts";
import { TimelineLayout } from "./timeline.ts";
import { TreeLayout } from "./tree.ts";
import { ObjectError } from "../error/error.ts";

/** The locale declarations are written in. */
const SOURCE = Localization.of(SOURCE_LOCALE, []);

/** Tasks nested under each other, with a state, two times and a declared timeline. */
const task = defineObject({
    name: "task",
    plural: "tasks",
    title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
    scope: space,
    fields: {
        title: field.string(),
        state: field.enum(["todo", "started", "done"]),
        dueAt: field.time().optional(),
        startsAt: field.time(),
        minutes: field.integer(),
    },
    access: { permissions: { read: through("scope", "read"), write: through("scope", "read") } },
    traits: [nested({ in: "self", optional: true, add: "write" })],
    presentable: {
        views: [
            {
                name: "schedule",
                title: t`Schedule`,
                layout: {
                    kind: "timeline",
                    start: "startsAt",
                    end: { duration: "minutes" },
                    unit: "millisecond",
                },
            },
        ],
    },
});

test("offer a table, a tree from the nesting, a board per enum, a calendar per time and the declared timeline", () => {
    expect(task.layouts).toEqual([
        { kind: "table" },
        { kind: "tree", id: "id", parent: "parentId" },
        { kind: "board", by: "state", empty: "show" },
        { kind: "calendar", by: "dueAt" },
        { kind: "calendar", by: "startsAt" },
        { kind: "timeline", start: "startsAt", end: { duration: "minutes" }, unit: "millisecond" },
    ]);
});

test("read a board's columns from its enum in declared order, and refuse a field of another type", () => {
    expect(
        BoardLayout.choices({ kind: "board", by: "state", empty: "show" }, task.fields, SOURCE),
    ).toEqual([
        { value: "todo", title: "todo" },
        { value: "started", title: "started" },
        { value: "done", title: "done" },
    ]);
    expect(() =>
        BoardLayout.choices({ kind: "board", by: "title", empty: "show" }, task.fields, SOURCE),
    ).toThrow(
        ObjectError.of(
            "DECLARATION_BROKEN",
            "board field 'title' is invalid: it is not an enum, a state or a reference to ordered objects",
        ),
    );
});

test("extend a list query with a board's order, a calendar's window and a timeline's window and order", () => {
    const query = { where: { title: { contains: "ship" } } };
    const window = { start: 100, end: 200 };

    expect({
        board: Layout.query({ kind: "board", by: "state", empty: "show" }, query),
        calendar: Layout.query({ kind: "calendar", by: "dueAt" }, query, window),
        timeline: Layout.query(
            {
                kind: "timeline",
                start: "startsAt",
                end: { duration: "minutes" },
                unit: "millisecond",
            },
            query,
            window,
        ),
        table: Layout.query({ kind: "table" }, query),
    }).toEqual({
        board: { ...query, orderBy: { state: "asc" } },
        calendar: { where: { AND: [query.where, { dueAt: { gte: 100, lt: 200 } }] } },
        timeline: {
            where: { AND: [query.where, { startsAt: { lt: 200 } }] },
            orderBy: { startsAt: "asc" },
        },
        table: query,
    });
});

test("nest rows depth first under their parents, and read a timeline's periods in milliseconds", () => {
    // nest a child under its root, an orphan as a second root
    const rows = [
        { id: "b", parent: "a" },
        { id: "a", parent: null },
        { id: "c", parent: "missing" },
    ];
    const placed = TreeLayout.nest({ kind: "tree", id: "id", parent: "parent" }, rows);

    // read periods by an end field, by a duration in microseconds, and none for an open row
    const byEnd = {
        kind: "timeline",
        start: "start",
        end: { field: "end" },
        unit: "millisecond",
    } as const;
    const byDuration = {
        kind: "timeline",
        start: "start",
        end: { duration: "length" },
        unit: "microsecond",
    } as const;
    expect({
        nested: placed.map((each) => [each.row["id"], each.isLast]),
        byEnd: TimelineLayout.period(byEnd, { start: 10, end: 25 }),
        byDuration: TimelineLayout.period(byDuration, { start: 10_000, length: 5000 }),
        open: TimelineLayout.period(byEnd, { start: 10, end: null }),
    }).toEqual({
        nested: [
            ["a", []],
            ["b", [true]],
            ["c", []],
        ],
        byEnd: { start: 10, end: 25 },
        byDuration: { start: 10, end: 15 },
        open: undefined,
    });
});

test("group a board by its columns, place a calendar's objects on days, and lay a timeline's objects on one axis", () => {
    // group tasks by state, an unknown state under no value
    const tasks = [
        { title: "a", state: "done", dueAt: 0 },
        { title: "b", state: "todo", dueAt: 86_400_000 + 5 },
        { title: "c", state: "lost", dueAt: 86_400_000 * 3 },
    ];
    const layout = { kind: "board", by: "state", empty: "show" } as const;
    const choices = BoardLayout.choices(layout, task.fields, SOURCE) ?? [];
    const groups = BoardLayout.group(layout, tasks, choices, SOURCE);

    // place them on days, and on an axis from the first start to the last end
    const days = CalendarLayout.days({ kind: "calendar", by: "dueAt" }, tasks, "UTC");
    const timeline = {
        kind: "timeline",
        start: "start",
        end: { field: "end" },
        unit: "millisecond",
    } as const;
    const axis = TimelineLayout.axis(timeline, [
        { id: "x", start: 100, end: 150 },
        { id: "y", start: 150, end: 300 },
        { id: "z", start: 120, end: null },
    ]);
    expect({
        groups: groups.map((group) => [group.choice.title, group.rows.map((row) => row["title"])]),
        days: days.map((day) => [day.date, day.rows.map((row) => row["title"])]),
        axis: {
            window: axis?.window,
            entries: axis?.entries.map((each) => [each.row["id"], each.start, each.end]),
        },
        open: TimelineLayout.axis(timeline, [{ id: "z", start: 120, end: null }]),
    }).toEqual({
        groups: [
            ["No value", ["c"]],
            ["todo", ["b"]],
            ["started", []],
            ["done", ["a"]],
        ],
        days: [
            ["1970-01-01", ["a"]],
            ["1970-01-02", ["b"]],
            ["1970-01-03", []],
            ["1970-01-04", ["c"]],
        ],
        axis: {
            window: { start: 100, end: 300 },
            entries: [
                ["x", 100, 150],
                ["y", 150, 300],
            ],
        },
        open: undefined,
    });
});

test("mark a timeline's failed objects, and its critical path through the children that set each end", () => {
    // a request whose read runs first and whose write, then its commit, sets the end
    const timeline = {
        kind: "timeline",
        start: "start",
        end: { field: "end" },
        unit: "millisecond",
        failed: { field: "status", value: "error" },
    } as const;
    const tree = { kind: "tree", id: "id", parent: "parent" } as const;
    const rows = [
        { id: "request", parent: null, start: 0, end: 100, status: "ok" },
        { id: "read", parent: "request", start: 5, end: 30, status: "ok" },
        { id: "cache", parent: "request", start: 10, end: 20, status: "error" },
        { id: "write", parent: "request", start: 40, end: 95, status: "ok" },
        { id: "commit", parent: "write", start: 80, end: 94, status: "ok" },
        { id: "log", parent: "write", start: 45, end: 50, status: "ok" },
    ];
    const critical = TimelineLayout.critical(timeline, tree, rows);

    expect({
        failed: rows.filter((row) => TimelineLayout.isFailed(timeline, row)).map((row) => row.id),
        critical: rows.filter((row) => critical.has(row)).map((row) => row.id),
    }).toEqual({
        failed: ["cache"],
        critical: ["request", "read", "write", "commit", "log"],
    });
});
