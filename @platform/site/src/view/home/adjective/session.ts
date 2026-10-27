/** What the log shows of the session: as the agent left it, reviewed by your agent, or fixed. */
export type Lane = "prepared" | "reviewed" | "fixed";

/** One change a step made in an app: the app's rail icon and what changed. */
export type Change = readonly [app: string, text: string];

/** A slip your agent finds in a step: what it is, and how it stands once fixed. */
export type Slip = {
    /** What is wrong, as your agent flags it. */
    text: string;
    /** How the slip stands after the fix, or undefined when it cannot be taken back. */
    fixed: string | undefined;
};

/** One step of the session: its app, what it did, how much, its changes, and any slip. */
export type Step = {
    /** The app the step worked in, as a rail icon. */
    app: string;
    /** What the step did, in a line. */
    text: string;
    /** How much the step changed. */
    count: string;
    /** The changes the step made, the first few. */
    changes: readonly Change[];
    /** The slip your agent finds in the step, or undefined when the step is right. */
    slip: Slip | undefined;
};

/** What you asked your agent on Thursday evening. */
export const REQUEST =
    "We launch Friday 9:00. Open signups, email the waitlist, publish the launch post and the press page, post in #launch, and get the demo requests onto the board and the calendar.";

/** The steps your agent took for the request, in order. */
export const steps: readonly Step[] = [
    {
        app: "table",
        text: "Opened signups",
        count: "1 change",
        changes: [["table", "Signups: status open, from Fri 09:00"]],
        slip: undefined,
    },
    {
        app: "mail",
        text: "Emailed the waitlist",
        count: "1,840 sent",
        changes: [
            ["mail", 'Sent "Destack opens Friday" to Waitlist · 1,840 people'],
            ["table", "Included 400 invited Thursday: ines@orbitlabs.eu, jonas@kettle.dev, …"],
        ],
        slip: { text: "400 already invited", fixed: undefined },
    },
    {
        app: "pages",
        text: 'Scheduled "Destack is live"',
        count: "Fri 09:00",
        changes: [
            ["pages", "/blog/launch: draft → publish at Fri 09:00"],
            ["pages", 'Linked "See pricing" to /pricing, still a draft'],
        ],
        slip: { text: "links to draft /pricing", fixed: "/pricing published" },
    },
    {
        app: "pages",
        text: "Published the press page",
        count: "1 change",
        changes: [["pages", "/press: draft → published"]],
        slip: undefined,
    },
    {
        app: "chat",
        text: "Scheduled the #launch post",
        count: "Fri 09:00 UTC",
        changes: [["chat", '#launch: "Destack is live 🚀" at Fri 09:00 UTC']],
        slip: { text: "11:00 your time", fixed: "moved to 09:00" },
    },
    {
        app: "tasks",
        text: "Boarded 9 demo requests",
        count: "9 cards",
        changes: [["tasks", 'Created DST-31 to DST-39 "Demo · Ines", …']],
        slip: undefined,
    },
    {
        app: "calendar",
        text: "Booked 9 demos",
        count: "9 events",
        changes: [["calendar", '"Demo with Ines" Mon 11:00, …']],
        slip: undefined,
    },
    {
        app: "tasks",
        text: 'Closed "Record the demo"',
        count: "1 change",
        changes: [["tasks", "DST-12: open → done, attached demo.mp4 from Oct 14"]],
        slip: { text: "demo.mp4 is last week's", fixed: "reopened" },
    },
];

/** The changes the session made: 1 + 1 send + 2 + 1 + 1 + 9 cards + 9 events + 1. */
export const CHANGE_COUNT = 25;
