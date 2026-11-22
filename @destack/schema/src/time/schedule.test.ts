import { expect, test } from "@destack/test";
import { Schedule, type ScheduleLink, type ScheduleTask } from "./schedule.ts";

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
