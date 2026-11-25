import { PlainDate } from "@destack/schema";
import { LABELS, PEOPLE, PROJECTS, type Task, TODAY } from "./state.ts";

/** The milliseconds of an hour. */
const HOUR = 3_600_000;

/** The launch's tasks by hand: long and short, right to left, under others, late and done. */
const WRITTEN: readonly (Pick<Task, "title" | "status"> & Partial<Task>)[] = [
    {
        title: "Record the demo",
        status: "doing",
        priority: "urgent",
        assignee: "grace",
        labels: ["story"],
        project: "launch",
        due: "2026-11-26",
        text: "Two minutes: the board, the calendar and the palette, recorded in one go.",
    },
    { title: "Write the script", status: "done", assignee: "grace", parent: "HAR-1" },
    { title: "Book the microphone", status: "doing", assignee: "jonas", parent: "HAR-1" },
    {
        title: "Pricing page says what each plan costs, including the yearly discount, the student plan, and what happens to a team that grows past its seats in the middle of a month",
        status: "doing",
        priority: "high",
        assignee: "ada",
        labels: ["site", "plan"],
        project: "launch",
        due: "2026-11-27",
    },
    {
        title: "Sign-up flow copy",
        status: "review",
        priority: "high",
        assignee: "jonas",
        labels: ["site"],
        project: "launch",
        due: "2026-11-24",
    },
    {
        title: "Billing terms",
        status: "review",
        priority: "medium",
        assignee: "max",
        labels: ["plan"],
        project: "shop",
        due: "2026-11-30",
    },
    {
        title: "ترجمة صفحة الأسعار إلى العربية",
        status: "todo",
        priority: "medium",
        assignee: "layla",
        labels: ["site"],
        project: "launch",
        due: "2026-12-02",
    },
    {
        title: "Two checkouts charge the card twice",
        status: "todo",
        priority: "urgent",
        labels: ["bug"],
        project: "shop",
    },
    { title: "Collect three quotes", status: "todo", assignee: "ada", labels: ["story"] },
    {
        title: "Cover paper samples",
        status: "backlog",
        priority: "low",
        labels: ["print"],
        project: "brand",
    },
    {
        title: "Draft the press note",
        status: "backlog",
        assignee: "ada",
        labels: ["story"],
        project: "launch",
    },
    { title: "Set up the domain", status: "done", assignee: "ada", labels: ["site"] },
    { title: "Second logo", status: "canceled", project: "brand" },
];

/** The words of the titles of tasks by the thousand. */
const WORDS = {
    verb: ["Draft", "Check", "Print", "Send", "Plan", "Fix", "Translate", "Measure", "Photograph"],
    noun: [
        "the poster",
        "the order form",
        "the index",
        "shelf labels",
        "the newsletter",
        "a margin",
    ],
    place: ["for Lisbon", "before the fair", "on phones", "in the shop", "with the printer", ""],
};

/** Pick a value, the same one for the same seed. */
function pick<Value>(values: readonly Value[], seed: number): Value | undefined {
    return values[seed % values.length];
}

/** Build tasks of the studio, those by hand first. */
export function sampleTasks(count: number, now: number): readonly Task[] {
    // pick the same things every time
    const statuses = ["backlog", "backlog", "todo", "doing", "done", "done", "canceled", "review"];
    const priorities = ["none", "low", "medium", "high", "none", "urgent", "medium"];

    return Array.from({ length: count }, (_task, i): Task => {
        // take a task by hand, else make one up
        const written = WRITTEN[i];
        const seed = i * 7 + Math.floor(i / 11);
        const label = pick(LABELS, seed);
        const words = [pick(WORDS.verb, seed), pick(WORDS.noun, i), pick(WORDS.place, seed + i)];
        const made: Partial<Task> =
            written ??
            ({
                title: words.filter((word) => word !== "").join(" "),
                status: pick(statuses, seed) ?? "todo",
                priority: pick(priorities, i) ?? "none",
                assignee: i % 4 === 0 ? null : (pick(PEOPLE, seed)?.id ?? null),
                labels: i % 3 === 0 || label === undefined ? [] : [label.value ?? ""],
                project: PROJECTS[i % 4]?.value ?? null,
                due: i % 5 === 0 ? PlainDate.add(TODAY, { days: (i % 40) - 10 }) : null,
            } satisfies Partial<Task>);
        const id = `HAR-${String(i + 1)}`;
        const created = now - (i + 1) * 5 * HOUR;

        return {
            title: "",
            text: "",
            status: "todo",
            priority: "none",
            assignee: null,
            labels: [],
            project: null,
            due: null,
            parent: null,
            ...made,
            id,
            created,
            events: [
                {
                    id: `${id} created`,
                    person: "ada",
                    at: created,
                    change: { property: "created", value: "" },
                },
            ],
        };
    });
}
