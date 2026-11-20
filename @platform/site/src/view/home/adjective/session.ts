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

/** What you asked your agent last night. */
export const REQUEST =
    "Tidy my inbox tonight: put what needs me on top, file receipts, archive the newsletters I never open, and send me a digest every morning at 8.";

/** The steps your agent took for the request, in order, all in preview until you keep them. */
export const steps: readonly Step[] = [
    {
        app: "mail",
        text: "Sorted by needs me",
        count: "12 threads",
        changes: [["mail", "Inbox: threads waiting on you first · Ines, Jonas, Mia"]],
        slip: undefined,
    },
    {
        app: "mail",
        text: "Filed receipts",
        count: "31 mails",
        changes: [
            ["mail", "31 mails from billing → Receipts"],
            ["table", "Receipts: 31 rows, each matched to its payment"],
        ],
        slip: undefined,
    },
    {
        app: "mail",
        text: "Archived newsletters",
        count: "214 mails",
        changes: [
            ["mail", "214 mails from 19 senders → Archive"],
            ["mail", 'Included Kai Brandt, "Onboarding sketch"'],
        ],
        slip: { text: "Kai's sketch isn't a newsletter", fixed: "back in your inbox" },
    },
    {
        app: "sync",
        text: "Set up a morning digest",
        count: "daily 08:00",
        changes: [
            ["sync", "digest-daily: every day at 08:00"],
            ["mail", "One mail a day: unread threads and tasks due"],
        ],
        slip: { text: "repeats your Monday page", fixed: "dropped" },
    },
];

/** The changes the session made: 1 sort + 31 filed + 214 archived + 1 workflow. */
export const CHANGE_COUNT = 247;
