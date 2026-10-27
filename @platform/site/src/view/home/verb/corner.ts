/** What the waitlist promises: a short title and its line. */
export const perks: readonly (readonly [title: string, line: string])[] = [
    ["Early pricing", "Lock in the launch rate"],
    ["First invites", "1,840 people, in batches from Thursday"],
    ["A direct line", "Book a call with Florian"],
];

/** The faces of the latest people to join the waitlist: an initial and a tint. */
export const joiners: readonly (readonly [initial: string, tint: string])[] = [
    ["M", "#c4683a"],
    ["J", "#3f7d88"],
    ["T", "#6f7a3f"],
    ["L", "#6b5ca5"],
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

/** The Destack launch plan's checklist: the item and whether it is done. */
export const launch: readonly (readonly [item: string, isDone: boolean])[] = [
    ["Freeze the pricing page", true],
    ["Record the demo", false],
    ["Email the waitlist", false],
    ["Publish the launch post", false],
];

/** A task's state, from not started to done. */
export type TaskState = "todo" | "started" | "review" | "done";

/** A launch task: its key, title, state, owner, owner's colour, and due date. */
export type Task = readonly [
    id: string,
    title: string,
    state: TaskState,
    owner: string,
    tint: string,
    due: string,
];

/** The launch tasks. */
export const tasks: readonly Task[] = [
    ["DST-12", "Pricing page", "done", "F", "#2f7d8c", "Oct 20"],
    ["DST-14", "Record the demo", "started", "F", "#2f7d8c", "Oct 22"],
    ["DST-16", "Email the waitlist", "review", "A", "#6b5ca5", "Oct 23"],
    ["DST-13", "Turn on pricing", "todo", "F", "#2f7d8c", "Oct 24"],
    ["DST-15", "Draft the launch post", "todo", "A", "#6b5ca5", "Oct 23"],
    ["DST-17", "Open signups", "todo", "F", "#2f7d8c", "Oct 24"],
    ["DST-18", "Find out who owns the Zapier account", "todo", "A", "#6b5ca5", "Oct 24"],
    ["DST-19", "Cancel the other nine subscriptions", "todo", "F", "#2f7d8c", "Oct 27"],
];

/** A message in the launch channel: who wrote it, when, and what they said. */
export type Message = readonly [author: "Florian" | "Agent" | "Zapier", time: string, text: string];

/** The launch channel's messages. */
export const messages: readonly Message[] = [
    ["Florian", "09:12", "Ines from Orbit Labs booked the demo. Wednesday at 11, apparently."],
    ["Agent", "09:13", "Added it to DST-14 and the launch page: Wed 11:00."],
    ["Agent", "09:40", "Drafted the Hacker News post. Two titles, the second one is spicier."],
    ["Florian", "10:02", "The first one. We have a reputation to build."],
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
    [0, 9, 9.5, "Standup", false],
    [0, 14, 15, "Pricing review", false],
    [1, 10, 11, "Bookable", true],
    [1, 15, 16.5, "Bookable", true],
    [2, 11, 12, "Demo · Ines", false],
    [2, 13, 14, "Bookable", true],
    [3, 9, 10, "Invites · 400", false],
    [3, 16, 17, "Bookable", true],
    [4, 9, 10, "Go live · DST-17", false],
    [4, 11, 12, "Bookable", true],
    [4, 16, 17, "Retro · why it took nine apps", false],
];

/** A signup's state, invited or still waiting. */
export type SignupState = "invited" | "approved" | "waiting";

/** A signup: their address, where they came from, when they joined, their state, and a note. */
export type Signup = readonly [
    email: string,
    source: string,
    joined: string,
    state: SignupState,
    note: string,
];

/** The latest signups: the address. */
export const signups: readonly Signup[] = [
    ["maya@northwind.io", "Launch post", "2 min ago", "waiting", ""],
    ["jonas@kettle.dev", "Website", "14 min ago", "waiting", ""],
    ["tara@fieldnotes.co", "Friend", "1 h ago", "waiting", ""],
    ["leo@brightpath.app", "Website", "3 h ago", "waiting", ""],
    ["sam@quietly.so", "Launch post", "Yesterday", "invited", ""],
    ["noor@papertrail.fm", "Demo day", "Yesterday", "invited", ""],
    ["ines@orbitlabs.eu", "Website", "Oct 18", "invited", ""],
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
        "Re: Destack demo",
        "Wednesday at 11 works. Will you be demoing it from Notion?",
        "10:14",
        true,
    ],
    [
        "Kai Brandt",
        "#2f7d8c",
        "Press kit?",
        "Could you send the logo and a screenshot before Friday?",
        "09:31",
        true,
    ],
    [
        "Tara Okafor",
        "#4f8a5b",
        "Waitlist question",
        "Do invites go out by signup date or at random?",
        "08:52",
        false,
    ],
    [
        "Calendly",
        "#006bff",
        "New event: Destack demo",
        "Ines Ferreira booked Wed 22 Oct, 11:00 to 11:30",
        "Oct 20",
        false,
    ],
    [
        "Zapier",
        "#ff4f00",
        "Your Zap was turned off",
        "Welcome new signups hit an error too many times in a row",
        "Oct 20",
        false,
    ],
    [
        "Linear",
        "#5e6ad2",
        "DST-14 moved to Wednesday",
        "Agent changed the due date of Record the demo",
        "Oct 19",
        false,
    ],
];

/** A press contact's state, from pitched to scheduled. */
export type PressState = "pitched" | "replied" | "scheduled";

/** The press list: each person, their outlet, the pitch's state, the last contact, and a tint. */
export const press: readonly (readonly [
    name: string,
    outlet: string,
    state: PressState,
    last: string,
    tint: string,
])[] = [
    ["Ines Ferreira", "Orbit Labs Weekly", "scheduled", "Oct 22", "#a0485f"],
    ["Kai Brandt", "Kettle Podcast", "replied", "Oct 21", "#2f7d8c"],
    ["Lena Okoye", "Fieldnotes", "pitched", "Oct 20", "#4f8a5b"],
    ["Ravi Patel", "Brightpath Daily", "pitched", "Oct 19", "#6b5ca5"],
    ["Mia Sørensen", "Quietly", "replied", "Oct 18", "#c4683a"],
];

/** The launch plan's columns: each name and its notes, a task's id or a plain note. */
export const board: readonly (readonly [column: string, notes: readonly string[]])[] = [
    ["Before", ["DST-12", "DST-14", "DST-16"]],
    ["Launch day", ["DST-17", "Post on Hacker News", "DST-15"]],
    ["After", ["DST-19", "Delete this board"]],
];

/** The welcome workflow's steps: the app it runs in, what it does, and the detail it shows. */
export const welcome: readonly (readonly [app: string, step: string, detail: string])[] = [
    ["Waitlist", "New signup", "When someone joins the waitlist"],
    ["Slack", "Post in #launch", "“New signup: {email} from {source}”"],
    ["Gmail", "Send welcome mail", "To {email}: “You're on the list”"],
];

/** Your space's workflows: each name, what starts it, its runs this month, and its last run. */
export const workflows: readonly (readonly [
    name: string,
    trigger: string,
    runs: string,
    last: string,
])[] = [
    ["welcome-signup", "on signup.create", "1,840", "2 min ago"],
    ["invite-400", "Thu 09:00", "4", "Oct 23"],
    ["press-followup", "on mail.reply from press", "12", "Oct 21"],
    ["friday-digest", "Fri 17:00", "3", "Oct 17"],
];
