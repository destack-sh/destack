import type { ItemMark } from "../figure/item";

/** One tool you rent automations in: its job, mark and name, its history page, and its plan. */
export type Tool = {
    job: string;
    mark: ItemMark;
    tint: string;
    name: string;
    page: string;
    url: string;
    plan: string;
    counts: readonly [runs: string, failed: string, traced: string];
};

/** One step of a traced run: who ran it, what it did, why it was allowed, and when. */
export type Step = {
    icon: string;
    app: string;
    name: string;
    allowed: string;
    depth: number;
    start: number;
    end: number;
};

/** One automation of your space: its job, app, the run traced, and its counts today. */
export type Flow = {
    job: string;
    icon: string;
    name: string;
    title: string;
    when: string;
    steps: readonly Step[];
    counts: readonly [runs: string, failed: string, traced: string];
};

/** Return a step. */
function step(
    icon: string,
    app: string,
    name: string,
    allowed: string,
    depth: number,
    start: number,
    end: number,
): Step {
    return { icon, app, name, allowed, depth, start, end };
}

/** The tools you rent automations in, each with its own history and its own plan. */
export const tools: readonly Tool[] = [
    {
        job: "Glue",
        mark: { icon: "zapier", tint: "#ff4f00" },
        tint: "#ff4f00",
        name: "Zapier",
        page: "Demo booked → notes and issue",
        url: "zapier.com/editor/248117/draft",
        plan: "Professional plan",
        counts: ["14", "2", "—"],
    },
    {
        job: "Assistant",
        mark: { icon: "openai", tint: "#10a37f" },
        tint: "#10a37f",
        name: "ChatGPT",
        page: "Tasks",
        url: "chatgpt.com/tasks",
        plan: "Plus plan",
        counts: ["3", "1", "—"],
    },
    {
        job: "Booking",
        mark: { icon: "calendly", tint: "#006bff" },
        tint: "#006bff",
        name: "Calendly",
        page: "Workflows",
        url: "calendly.com/app/workflows",
        plan: "Standard plan",
        counts: ["6", "1", "—"],
    },
    {
        job: "Tasks",
        mark: { icon: "linear", tint: "#5e6ad2" },
        tint: "#5e6ad2",
        name: "Linear",
        page: "Workflow · Settings",
        url: "linear.app/destack/settings/teams/DST/workflow",
        plan: "Basic plan",
        counts: ["9", "1", "—"],
    },
    {
        job: "Chat",
        mark: { icon: "slack", tint: "#4a154b" },
        tint: "#4a154b",
        name: "Slack",
        page: "Workflow Builder",
        url: "app.slack.com/workflow-builder/T04D3ST4CK",
        plan: "Pro plan",
        counts: ["4", "1", "—"],
    },
    {
        job: "Docs",
        mark: { icon: "notion", tint: "#191919" },
        tint: "#191919",
        name: "Notion",
        page: "Launch tasks",
        url: "notion.so/destack/Launch-tasks",
        plan: "Plus plan",
        counts: ["5", "0", "—"],
    },
    {
        job: "Mail",
        mark: { icon: "gmail", tint: "#ea4335" },
        tint: "#ea4335",
        name: "Gmail",
        page: "Filters",
        url: "mail.google.com/mail/#settings/filters",
        plan: "Business Starter",
        counts: ["?", "?", "—"],
    },
    {
        job: "Code",
        mark: { icon: "github", tint: "#181717" },
        tint: "#181717",
        name: "GitHub",
        page: "Show yearly prices · build #411",
        url: "github.com/destack/site/actions/runs/411",
        plan: "Team plan",
        counts: ["11", "1", "—"],
    },
];

/** The automations of your space, each with one run traced through every app it touched. */
export const flows: readonly Flow[] = [
    {
        job: "Glue",
        icon: "sync",
        name: "Workflows",
        title: "Guest books a demo",
        when: "Today 10:02",
        counts: ["14", "0", "14"],
        steps: [
            step(
                "calendar",
                "Calendar",
                "Guest books Thu 14:00",
                "your booking page is public",
                0,
                0,
                9,
            ),
            step("storage", "Database", "Save the event", "Calendar owns events", 1, 2, 6),
            step(
                "calendar",
                "Calendar",
                "Announce event.booked",
                "Calendar declares the topic",
                1,
                6,
                8,
            ),
            step(
                "file",
                "Notetaker",
                "Prepare meeting notes",
                "it listens to event.booked",
                0,
                10,
                31,
            ),
            step(
                "calendar",
                "Notetaker",
                "Read the event",
                "it declared title, time, attendees",
                1,
                11,
                16,
            ),
            step("storage", "Database", "Save the note", "Notetaker owns notes", 1, 17, 20),
            step("agent", "Your agent", "Read the notes", "it acts as you", 0, 21, 24),
            step("tasks", "Tasks", "Create a follow-up task", "your agent acts as you", 0, 24, 29),
            step("sync", "Workflows", "Remind you Thu 09:00", "Tasks scheduled it", 1, 27, 29),
            step(
                "notify",
                "Notifications",
                "Tell you it is done",
                "you follow the task",
                1,
                29,
                31,
            ),
        ],
    },
    {
        job: "Assistant",
        icon: "agent",
        name: "Your agent",
        title: "Morning brief",
        when: "Today 08:00",
        counts: ["3", "0", "3"],
        steps: [
            step("sync", "Workflows", "Wake your agent at 08:00", "you scheduled it", 0, 0, 2),
            step("agent", "Your agent", "Write the morning brief", "it acts as you", 0, 2, 41),
            step(
                "calendar",
                "Calendar",
                "Read today's events",
                "you can read your calendar",
                1,
                3,
                7,
            ),
            step("mail", "Mail", "Read unread threads", "you can read your mail", 1, 7, 15),
            step(
                "file",
                "Notetaker",
                "Read yesterday's notes",
                "you can read your notes",
                1,
                15,
                19,
            ),
            step("pages", "Pages", "Write “Brief · Tue”", "you can write in Pages", 1, 34, 39),
            step(
                "notify",
                "Notifications",
                "Send it to your phone",
                "you follow your briefs",
                1,
                39,
                41,
            ),
        ],
    },
    {
        job: "Booking",
        icon: "calendar",
        name: "Calendar",
        title: "Remind the guest",
        when: "Today 13:00",
        counts: ["6", "0", "6"],
        steps: [
            step("sync", "Workflows", "Wake at 13:00", "Calendar scheduled it", 0, 0, 1),
            step(
                "calendar",
                "Calendar",
                "Find Thu 14:00 with Guest",
                "Calendar owns events",
                0,
                1,
                4,
            ),
            step("mail", "Mail", "Send the reminder", "Calendar declared it sends mail", 0, 4, 12),
            step(
                "storage",
                "Database",
                "Note the reminder on the event",
                "Calendar owns events",
                1,
                10,
                12,
            ),
        ],
    },
    {
        job: "Tasks",
        icon: "tasks",
        name: "Tasks",
        title: "Close task on merge",
        when: "Today 10:44",
        counts: ["9", "0", "9"],
        steps: [
            step("source", "Forge", "Merge pricing-page", "you can merge in Forge", 0, 0, 6),
            step("source", "Forge", "Announce change.merged", "Forge declares the topic", 1, 5, 6),
            step("tasks", "Tasks", "Close DST-12", "it listens to change.merged", 0, 6, 11),
            step("chat", "Chat", "Post in #launch", "Tasks declared it posts", 0, 11, 14),
        ],
    },
    {
        job: "Chat",
        icon: "chat",
        name: "Chat",
        title: "New signup to #launch",
        when: "Today 10:05",
        counts: ["4", "0", "4"],
        steps: [
            step("table", "Waitlist", "Add ana@studio.dev", "your signup form is public", 0, 0, 5),
            step(
                "table",
                "Waitlist",
                "Announce signup.created",
                "Waitlist declares the topic",
                1,
                4,
                5,
            ),
            step("chat", "Chat", "Post in #launch", "it listens to signup.created", 0, 5, 9),
        ],
    },
    {
        job: "Docs",
        icon: "pages",
        name: "Pages",
        title: "Status change",
        when: "Today 10:20",
        counts: ["5", "0", "5"],
        steps: [
            step("pages", "Pages", "Set “Launch plan” to Done", "you can edit the page", 0, 0, 4),
            step("pages", "Pages", "Announce page.updated", "Pages declares the topic", 1, 3, 4),
            step("notify", "Notifications", "Tell the owner", "they follow the page", 0, 4, 6),
        ],
    },
    {
        job: "Mail",
        icon: "mail",
        name: "Mail",
        title: "Invoice to Files",
        when: "Today 09:12",
        counts: ["7", "0", "7"],
        steps: [
            step("mail", "Mail", "Receive “Invoice #2031”", "mail on your domain", 0, 0, 3),
            step("mail", "Mail", "Match the invoices rule", "your rule", 1, 3, 4),
            step(
                "bucket",
                "Files",
                "Save invoice-2031.pdf",
                "Mail declared it stores files",
                0,
                4,
                10,
            ),
            step("tasks", "Tasks", "Create “Pay invoice #2031”", "your rule", 0, 10, 13),
        ],
    },
    {
        job: "Code",
        icon: "source",
        name: "Forge",
        title: "Build on push",
        when: "Today 10:41",
        counts: ["11", "0", "11"],
        steps: [
            step("source", "Forge", "Receive push to main", "you can push to the repo", 0, 0, 2),
            step("hosts", "Hosting", "Build the site", "Forge declared it builds", 0, 2, 26),
            step("telemetry", "Telemetry", "Record the build", "every service reports", 1, 24, 26),
            step("source", "Forge", "Mark the change passed", "Forge owns changes", 0, 26, 28),
        ],
    },
];

/** The totals of each state's table: the name, then runs, failed and traced. */
export const totals = {
    stacked: ["8 tools", "52+", "7+", "0"],
    destacked: ["1 space", "59", "0", "59"],
} as const;

/** Return the milliseconds a flow's traced run takes, from its first step to its last. */
export function durationOf(flow: Flow): number {
    return Math.max(...flow.steps.map((each) => each.end));
}

/** The engine services a run passes through, which the path leaves out to name only apps. */
const services = new Set([
    "Database",
    "Workflows",
    "Notifications",
    "Files",
    "Hosting",
    "Telemetry",
]);

/** Return the apps a flow's run went through, each once, in order. */
export function pathOf(flow: Flow): readonly (readonly [icon: string, app: string])[] {
    const seen = new Map<string, string>();
    for (const each of flow.steps) {
        if (!seen.has(each.app) && !services.has(each.app)) {
            seen.set(each.app, each.icon);
        }
    }

    return [...seen].map(([app, icon]) => [icon, app] as const);
}
