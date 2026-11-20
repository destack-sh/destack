/** The Monday page's overview: a short title and its line. */
export const overview: readonly (readonly [title: string, line: string])[] = [
    ["Promised", "3 things you said you'd do"],
    ["Due", "4 tasks before Friday"],
    ["Free", "6 hours to do them"],
];

/** The faces of the people waiting on a reply from you: an initial and a tint. */
export const owed: readonly (readonly [initial: string, tint: string])[] = [
    ["I", "#a0485f"],
    ["J", "#3f7d88"],
    ["M", "#c4683a"],
];

/** The month the booking page shows, October 2025, with its free and chosen days. */
export const month = {
    days: 31,
    first: 3,
    open: [21, 22, 23, 27, 28, 30, 31],
    today: 20,
    chosen: 28,
};

/** The free times on the chosen day. */
export const times: readonly string[] = ["09:30", "10:00", "11:30", "15:00", "16:30"];

/** The open points on Ines's page: the point and whether it is settled. */
export const points: readonly (readonly [point: string, isDone: boolean])[] = [
    ["Sign off the booking flow", true],
    ["Send the estimate for phase 2", false],
    ["Decide on card payments", false],
    ["Move the app to her domain", false],
];

/** A task's state, from not started to done. */
export type TaskState = "todo" | "started" | "review" | "done";

/** A task: its key, title, state, owner, owner's colour, and due date. */
export type Task = readonly [
    id: string,
    title: string,
    state: TaskState,
    owner: string,
    tint: string,
    due: string,
];

/** Your tasks, yours and your agent's. */
export const tasks: readonly Task[] = [
    ["T-12", "Ship the booking fix", "done", "F", "#2f7d8c", "Oct 20"],
    ["T-14", "Send Ines the estimate", "started", "F", "#2f7d8c", "Oct 22"],
    ["T-16", "Match October's receipts", "review", "A", "#6b5ca5", "Oct 23"],
    ["T-13", "Invoice October", "todo", "F", "#2f7d8c", "Oct 24"],
    ["T-15", "Draft the reply to Jonas", "todo", "A", "#6b5ca5", "Oct 21"],
    ["T-17", "Renew my passport", "todo", "F", "#2f7d8c", "Oct 23"],
    ["T-18", "Find out what the £19 charge is", "todo", "A", "#6b5ca5", "Oct 24"],
    ["T-19", "Cancel the subscriptions I don't use", "todo", "F", "#2f7d8c", "Oct 27"],
];

/** A message in the channel you share with Ines: who wrote it, when, and what they said. */
export type Message = readonly [author: "Florian" | "Agent" | "Ines", time: string, text: string];

/** The messages in the channel you share with Ines. */
export const messages: readonly Message[] = [
    ["Ines", "09:12", "Could we take cards at booking? Half my clients ask for it."],
    ["Agent", "09:13", "Added it to T-14 and her page: card payments, to decide Wednesday."],
    ["Agent", "09:40", "Drafted the estimate for phase 2. Two options, the second adds cards."],
    ["Florian", "10:02", "The second one. Send it after our call."],
];

/** A block of your week. */
export type WeekBlock = readonly [
    day: number,
    start: number,
    end: number,
    title: string,
    isBookable: boolean,
];

/** Your week of October 20. */
export const week: readonly WeekBlock[] = [
    [0, 9, 11, "Focus", false],
    [0, 11.5, 12, "Call · Ines", false],
    [0, 12.5, 13.5, "Run", false],
    [0, 14, 15, "Booking fix · T-12", false],
    [0, 15.5, 16.5, "Bookable", true],
    [1, 9, 11, "Focus", false],
    [1, 11.5, 12.5, "Lunch · Jonas", false],
    [1, 14, 15.5, "Estimate · T-14", false],
    [1, 15.5, 16.5, "Bookable", true],
    [2, 9, 11, "Focus", false],
    [2, 11, 12, "Review · Ines", false],
    [2, 13, 14, "Bookable", true],
    [2, 15, 16, "Receipts · T-16", false],
    [3, 8.5, 9.5, "Passport · T-17", false],
    [3, 10, 12, "Focus", false],
    [3, 12.5, 13.5, "Run", false],
    [3, 16, 17, "Bookable", true],
    [4, 9, 10, "Invoices · T-13", false],
    [4, 11, 12, "Bookable", true],
    [4, 13, 14, "Accounts · Mia", false],
    [4, 16, 17, "Weekly review", false],
];

/** A commitment's state: still open, waiting on someone else, or kept. */
export type CommitmentState = "open" | "waiting" | "kept";

/** A commitment: who it is to, what you said, where you said it, its state, and a note. */
export type Commitment = readonly [
    person: string,
    what: string,
    said: string,
    state: CommitmentState,
    note: string,
];

/** What you said you'd do, in mail and chat, newest first. */
export const commitments: readonly Commitment[] = [
    ["Ines Ferreira", "Send the phase 2 estimate", "Mail · Oct 17", "open", ""],
    ["Jonas Weber", "Tell him about Saturday", "Chat · Oct 18", "open", ""],
    ["Mia Sørensen", "Send October's receipts", "Mail · Oct 14", "open", ""],
    ["Kai Brandt", "Look at his onboarding sketch", "Mail · Oct 10", "waiting", ""],
    ["Ines Ferreira", "Fix the double booking", "Call · Oct 13", "kept", ""],
    ["Lena Okoye", "Send the handover notes", "Mail · Oct 9", "kept", ""],
    ["Mia Sørensen", "Sign the engagement letter", "Mail · Oct 2", "kept", ""],
];

/** A mail: its sender, their colour, subject, preview, time, and whether it is unread. */
export type Mail = readonly [
    sender: string,
    tint: string,
    subject: string,
    preview: string,
    time: string,
    isUnread: boolean,
];

/** The mail in the inbox, newest first. */
export const mails: readonly Mail[] = [
    [
        "Ines Ferreira",
        "#a0485f",
        "Re: Phase 2",
        "Wednesday at 11 works. Can you bring the estimate?",
        "10:14",
        true,
    ],
    [
        "Jonas Weber",
        "#3f7d88",
        "Saturday?",
        "Are you in? We'd leave at 8 and be back by dinner.",
        "09:31",
        true,
    ],
    [
        "Mia Sørensen",
        "#c4683a",
        "October receipts",
        "Could you send them before Friday? I'll file the return next week.",
        "08:52",
        false,
    ],
    [
        "GitHub",
        "#24292f",
        "[booking] Fix double booking (#48)",
        "Merged into main by florian",
        "Oct 20",
        false,
    ],
    [
        "Stripe",
        "#635bff",
        "Your payout is on its way",
        "It arrives in your account on Oct 21",
        "Oct 20",
        false,
    ],
    [
        "Figma",
        "#a259ff",
        "Your receipt from Figma",
        "Professional plan, 1 editor, monthly",
        "Oct 19",
        false,
    ],
    [
        "Kai Brandt",
        "#5a7a2e",
        "Onboarding sketch",
        "No rush, but I'd love your take before I build it.",
        "Oct 19",
        false,
    ],
    [
        "Hetzner",
        "#d50c2d",
        "Invoice for September",
        "Your invoice is ready to download",
        "Oct 17",
        false,
    ],
    [
        "Vercel",
        "#171717",
        "Failed production deployment",
        "booking: the build failed on main",
        "Oct 17",
        false,
    ],
    ["Notion", "#191919", "Your receipt from Notion", "Plus plan, monthly", "Oct 16", false],
    [
        "Trainline",
        "#01c3a7",
        "Your e-ticket",
        "Return, Saturday 25 October, 08:04",
        "Oct 15",
        false,
    ],
];

/** Whose turn it is with a person: yours to reply, theirs, or nobody's. */
export type ReplyState = "owe" | "waiting" | "clear";

/** The people you deal with: name, relation, whose turn it is, the last mail, and a tint. */
export const people: readonly (readonly [
    name: string,
    relation: string,
    state: ReplyState,
    last: string,
    tint: string,
])[] = [
    ["Ines Ferreira", "Client · booking app", "owe", "Oct 22", "#a0485f"],
    ["Jonas Weber", "Friend", "owe", "Oct 21", "#3f7d88"],
    ["Mia Sørensen", "Accountant", "owe", "Oct 20", "#c4683a"],
    ["Kai Brandt", "Designer", "waiting", "Oct 19", "#5a7a2e"],
    ["Lena Okoye", "Former client", "clear", "Oct 9", "#4f8a5b"],
    ["Ravi Patel", "Climbing", "clear", "Oct 4", "#6b5ca5"],
];

/** Your week's board: each column and its notes, a task's id or a plain note. */
export const board: readonly (readonly [column: string, notes: readonly string[]])[] = [
    ["This week", ["T-14", "T-16", "T-13"]],
    ["Waiting on", ["Kai's sketch", "Ines on cards"]],
    ["Later", ["T-19", "Plan the winter trip"]],
];

/** The booking automation's steps: the app it runs in, what it does, and the detail it shows. */
export const bookingFlow: readonly (readonly [app: string, step: string, detail: string])[] = [
    ["Booking page", "New booking", "When someone books a call with you"],
    ["Google Sheets", "Add a row", "“{name}, {email}, {time}” to People"],
    ["Gmail", "Send an email", "To {email}: “See you {time}”"],
];

/** Your space's workflows: each name, what starts it, its runs this month, and its last run. */
export const workflows: readonly (readonly [
    name: string,
    trigger: string,
    runs: string,
    last: string,
])[] = [
    ["file-receipts", "on mail.create from billing", "31", "2 min ago"],
    ["owe-reply", "daily 08:00", "20", "today"],
    ["weekly-review", "Fri 16:00", "3", "Oct 17"],
    ["invoice-month", "1st of the month", "1", "Oct 1"],
];

/** Your workflows' latest runs: the workflow, its subject, its steps, its duration and when. */
export const runs: readonly (readonly [
    workflow: string,
    subject: string,
    steps: number,
    duration: string,
    when: string,
])[] = [
    ["file-receipts", "Figma · £12.00", 3, "0.8 s", "2 min ago"],
    ["owe-reply", "3 people waiting", 2, "1.4 s", "08:00"],
    ["file-receipts", "Notion · £8.00", 3, "0.7 s", "Oct 16"],
    ["file-receipts", "Hetzner · £16.40", 3, "0.9 s", "Oct 17"],
    ["weekly-review", "Week of Oct 13", 4, "6.2 s", "Oct 17"],
    ["invoice-month", "September · 3 clients", 3, "2.1 s", "Oct 1"],
];
