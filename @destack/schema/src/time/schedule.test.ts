import { expect, test } from "@destack/test";
import { WorkCalendar, WorkWeek } from "./calendar.ts";
import { Schedule, type ScheduleLink, type ScheduleTask } from "./schedule.ts";
import type { Weekday } from "./plain.ts";

/** The milliseconds of a day. */
const DAY = 86_400_000;

/** A launch plan in days: design, build and copy lead to a launch, a review stands apart. */
const PLAN: readonly ScheduleTask[] = [
    { key: "design", start: 0, end: 3 * DAY },
    { key: "build", start: 3 * DAY, end: 10 * DAY },
    { key: "copy", start: 8 * DAY, end: 14 * DAY },
    { key: "launch", start: 14 * DAY, end: 14 * DAY },
    { key: "review", start: 3 * DAY, end: 5 * DAY },
];

/** The plan's links: copy starts five days into the build, and both lead to the launch. */
const LINKS: readonly ScheduleLink[] = [
    { from: "design", to: "build", type: "finish-to-start", lag: 0 },
    { from: "build", to: "copy", type: "start-to-start", lag: 5 * DAY },
    { from: "build", to: "launch", type: "finish-to-start", lag: 0 },
    { from: "copy", to: "launch", type: "finish-to-start", lag: 0 },
];

/** Write tasks as their keys and their starts and ends in days. */
function daysOf(tasks: readonly ScheduleTask[]): [string, number, number][] {
    return tasks.map((task) => [task.key, task.start / DAY, task.end / DAY]);
}

test("find the critical path and each task's slack before the schedule's end", () => {
    // every linked task drives the launch, and the review may slip until the launch
    const schedule = new Schedule(PLAN, LINKS);

    expect({
        order: schedule.tasks.map((task) => task.key),
        critical: PLAN.filter((task) => schedule.isCritical(task.key)).map((task) => task.key),
        slack: PLAN.map((task) => schedule.slack(task.key) / DAY),
    }).toEqual({
        order: ["design", "review", "build", "copy", "launch"],
        critical: ["design", "build", "copy", "launch"],
        slack: [0, 0, 0, 0, 9],
    });
});

test("push the tasks that wait on a moved task by their links' types and lags, leaving the rest", () => {
    // design two days later pushes the build, the copy five days into it, and the launch after both
    const moved = new Schedule(PLAN, LINKS).move([{ key: "design", start: 2 * DAY, end: 5 * DAY }]);

    expect(daysOf(moved)).toEqual([
        ["design", 2, 5],
        ["build", 5, 12],
        ["copy", 10, 16],
        ["launch", 16, 16],
    ]);
});

test("leave the tasks that wait on a task moved earlier where they are", () => {
    // design a day earlier frees the build, which stays at its own dates
    const moved = new Schedule(PLAN, LINKS).move([{ key: "design", start: -DAY, end: 2 * DAY }]);

    expect(daysOf(moved)).toEqual([["design", -1, 2]]);
});

test("end a task with another's finish or start by finish-to-finish and start-to-finish links", () => {
    // a two-day check ending with a five-day write, and a day of notes ending as the write starts
    const tasks: readonly ScheduleTask[] = [
        { key: "write", start: 0, end: 5 * DAY },
        { key: "check", start: 0, end: 2 * DAY },
        { key: "notes", start: -4 * DAY, end: -3 * DAY },
    ];
    const links: readonly ScheduleLink[] = [
        { from: "write", to: "check", type: "finish-to-finish", lag: 0 },
        { from: "write", to: "notes", type: "start-to-finish", lag: 0 },
    ];
    const moved = new Schedule(tasks, links).move([{ key: "write", start: DAY, end: 6 * DAY }]);

    expect(daysOf(moved)).toEqual([
        ["write", 1, 6],
        ["check", 4, 6],
        ["notes", 0, 1],
    ]);
});

/** Order the plan with a link from its launch back to its design. */
function cycleOf(): Schedule<ScheduleTask> {
    return new Schedule(PLAN, [
        ...LINKS,
        { from: "launch", to: "design", type: "finish-to-start", lag: 0 },
    ]);
}

/** Order the plan with a link to a task it lacks. */
function missingOf(): Schedule<ScheduleTask> {
    return new Schedule(PLAN, [{ from: "build", to: "ship", type: "finish-to-start", lag: 0 }]);
}

/** Order the plan with its design twice. */
function repeatedOf(): Schedule<ScheduleTask> {
    return new Schedule([...PLAN, { key: "design", start: 0, end: DAY }], LINKS);
}

test("allow one link per pair of tasks the schedule has, closing no cycle", () => {
    // a review before the launch, a loop back, a self link, a missing task, and a second design link
    const schedule = new Schedule(PLAN, LINKS);
    const links: readonly ScheduleLink[] = [
        { from: "review", to: "launch", type: "finish-to-start", lag: 0 },
        { from: "launch", to: "review", type: "finish-to-start", lag: 0 },
        { from: "launch", to: "design", type: "finish-to-start", lag: 0 },
        { from: "design", to: "design", type: "finish-to-start", lag: 0 },
        { from: "build", to: "ship", type: "finish-to-start", lag: 0 },
        { from: "design", to: "build", type: "start-to-start", lag: 0 },
        { from: "build", to: "design", type: "finish-to-start", lag: 0 },
    ];

    // the review may lead to the launch or follow it, being unlinked, and nothing else joins
    expect(links.map((link) => schedule.allows(link))).toEqual([
        true,
        true,
        false,
        false,
        false,
        false,
        false,
    ]);
    expect([schedule.has("review"), schedule.has("ship")]).toEqual([true, false]);
});

test("refuse links that form a cycle or join a task the schedule lacks, and repeated tasks", () => {
    // the launch leading back to the design, a link to a missing ship task, and a second design
    expect(cycleOf).toThrow(
        new RangeError("schedule links form a cycle through 'design', 'build', 'copy', 'launch'"),
    );
    expect(missingOf).toThrow(
        new RangeError("schedule link 'build' to 'ship' joins a missing task"),
    );
    expect(repeatedOf).toThrow(new RangeError("schedule task 'design' is repeated"));
});

/** The weekend of Germany: Saturday and Sunday. */
const WEEKEND: readonly Weekday[] = [6, 7];

/** Read the instant of a day of November 2026 at an hour in UTC, the 2nd a Monday. */
function at(date: number, hour = 0): number {
    return Date.UTC(2026, 10, date, hour);
}

/** Write an instant as its day of November and its hour in UTC. */
function when(instant: number): string {
    const date = new Date(instant);

    return `${String(date.getUTCDate())} ${String(date.getUTCHours())}h`;
}

test("start a task waiting on one that ends on Friday on Monday, and end it at a working day's end", () => {
    // a design ending with Friday pushes a one-day build to Monday, and a three-day test to Thursday's end
    const tasks: readonly ScheduleTask[] = [
        { key: "design", start: at(2), end: at(5) },
        { key: "build", start: at(5), end: at(6) },
        { key: "test", start: at(5), end: at(10) },
    ];
    const links: readonly ScheduleLink[] = [
        { from: "design", to: "build", type: "finish-to-start", lag: 0 },
        { from: "build", to: "test", type: "finish-to-start", lag: 0 },
    ];
    const calendar = new WorkCalendar("UTC", WorkWeek.of(WEEKEND));
    const moved = new Schedule(tasks, links, { calendar }).move([
        { key: "design", start: at(3), end: at(7) },
    ]);

    expect(moved.map((task) => [task.key, when(task.start), when(task.end)])).toEqual([
        ["design", "3 0h", "7 0h"],
        ["build", "9 0h", "10 0h"],
        ["test", "10 0h", "13 0h"],
    ]);
});

test("push a task in working time over the weekend, keeping its working length", () => {
    // a design from Monday to Thursday pushes a three-day build that waits on it by a day
    const tasks: readonly ScheduleTask[] = [
        { key: "design", start: at(2), end: at(5) },
        { key: "build", start: at(5), end: at(10) },
    ];
    const links: readonly ScheduleLink[] = [
        { from: "design", to: "build", type: "finish-to-start", lag: 0 },
    ];
    const calendar = new WorkCalendar("UTC", WorkWeek.of(WEEKEND));
    const moved = new Schedule(tasks, links, { calendar }).move([
        { key: "design", start: at(3), end: at(6) },
    ]);

    expect(moved.map((task) => [task.key, when(task.start), when(task.end)])).toEqual([
        ["design", "3 0h", "6 0h"],
        ["build", "6 0h", "11 0h"],
    ]);
});

test("bound tasks by their constraints, a broken upper bound leaving negative slack", () => {
    // a build that may not start before the 6th, and a launch due by the 8th
    const tasks: readonly ScheduleTask[] = [
        { key: "design", start: 0, end: 3 * DAY },
        {
            key: "build",
            start: 3 * DAY,
            end: 5 * DAY,
            constraint: { type: "start-no-earlier-than", at: 6 * DAY },
        },
        {
            key: "launch",
            start: 5 * DAY,
            end: 6 * DAY,
            constraint: { type: "finish-no-later-than", at: 8 * DAY },
        },
    ];
    const links: readonly ScheduleLink[] = [
        { from: "design", to: "build", type: "finish-to-start", lag: 0 },
        { from: "build", to: "launch", type: "finish-to-start", lag: 0 },
    ];
    const schedule = new Schedule(tasks, links);
    const moved = schedule.move([{ key: "design", start: 0, end: 3 * DAY }]);

    expect({
        moved: daysOf(moved),
        slack: new Schedule(moved.length === 3 ? moved : tasks, links).slack("launch") / DAY,
    }).toEqual({
        moved: [
            ["design", 0, 3],
            ["build", 6, 8],
            ["launch", 8, 9],
        ],
        slack: -1,
    });
});

test("pull linked tasks earlier too in the auto mode, leaving unlinked tasks where they are", () => {
    // design moved two days earlier pulls the build and the launch along, the review stays
    const moved = new Schedule(PLAN, LINKS).move([{ key: "design", start: -2 * DAY, end: DAY }], {
        mode: "auto",
    });

    expect(daysOf(moved)).toEqual([
        ["design", -2, 1],
        ["build", 1, 8],
        ["copy", 6, 12],
        ["launch", 12, 12],
    ]);
});

test("read a task's free slack before the tasks waiting on it, its total slack before the end", () => {
    // the copy may slip until the launch, the review until the plan's end
    const schedule = new Schedule(PLAN, LINKS);

    expect(PLAN.map((task) => [task.key, schedule.freeSlack(task.key) / DAY])).toEqual([
        ["design", 0],
        ["build", 0],
        ["copy", 0],
        ["launch", 0],
        ["review", 9],
    ]);
});

test("start a milestone after a task ending on Friday on Monday, its end on its start", () => {
    // a design ending with Friday and a review milestone waiting on it
    const tasks: readonly ScheduleTask[] = [
        { key: "design", start: at(2), end: at(5) },
        { key: "review", start: at(5), end: at(5) },
    ];
    const links: readonly ScheduleLink[] = [
        { from: "design", to: "review", type: "finish-to-start", lag: 0 },
    ];
    const calendar = new WorkCalendar("UTC", WorkWeek.of(WEEKEND));
    const moved = new Schedule(tasks, links, { calendar }).move([
        { key: "design", start: at(3), end: at(7) },
    ]);

    expect(moved.map((task) => [task.key, when(task.start), when(task.end)])).toEqual([
        ["design", "3 0h", "7 0h"],
        ["review", "9 0h", "9 0h"],
    ]);
});
