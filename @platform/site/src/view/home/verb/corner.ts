/** Your posts: the title, the topic, the date, and the two tints of the cover. */
export const posts: readonly (readonly [
    title: string,
    topic: string,
    date: string,
    from: string,
    to: string,
])[] = [
    ["Introducing Destack", "Product", "Oct 18, 2025", "#e9b384", "#c4683a"],
    ["Notes from demo day", "Events", "Oct 9, 2025", "#9fc3c9", "#3f7d88"],
    ["Why I moved my tools home", "Tools", "Sep 30, 2025", "#c9c39f", "#6f7a3f"],
];

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

/** The pattern each post's cover lays over its tints, one per post. */
const patterns: readonly string[] = [
    "repeating-radial-gradient(circle at 78% 30%, rgb(255 255 255 / 22%) 0 2px, transparent 2px 16px)",
    "linear-gradient(90deg, rgb(255 255 255 / 16%) 1px, transparent 1px) 0 0 / 22px 22px, linear-gradient(rgb(255 255 255 / 16%) 1px, transparent 1px) 0 0 / 22px 22px",
    "repeating-linear-gradient(-28deg, rgb(255 255 255 / 18%) 0 2px, transparent 2px 13px)",
];

/** Paint a post's cover: its pattern over a gradient between its two tints. */
export function coverOf(index: number, from: string, to: string) {
    return `${patterns[index % patterns.length] ?? ""}, radial-gradient(circle at 20% 85%, rgb(255 255 255 / 25%), transparent 45%), linear-gradient(135deg, ${from}, ${to})`;
}

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

/** The launch tasks. */
export const tasks: readonly (readonly [
    id: string,
    title: string,
    state: TaskState,
    owner: string,
    tint: string,
    due: string,
])[] = [
    ["DST-12", "Pricing page", "done", "F", "#2f7d8c", "Oct 20"],
    ["DST-14", "Record the demo", "started", "F", "#2f7d8c", "Oct 22"],
    ["DST-16", "Email the waitlist", "review", "A", "#6b5ca5", "Oct 23"],
    ["DST-13", "Turn on pricing", "todo", "F", "#2f7d8c", "Oct 24"],
    ["DST-15", "Draft the launch post", "todo", "A", "#6b5ca5", "Oct 23"],
    ["DST-17", "Open signups", "todo", "F", "#2f7d8c", "Oct 24"],
];

/** The launch channel's messages: who wrote it, when, and what they said. */
export const messages: readonly (readonly [
    author: "Florian" | "Agent",
    time: string,
    text: string,
])[] = [
    ["Florian", "09:12", "Can we move the demo to Wednesday? Thursday is the waitlist send."],
    ["Agent", "09:13", "Moved it to Wed 11:00 and updated DST-14 and the launch plan."],
    [
        "Agent",
        "09:40",
        "First invite batch is drafted. 1,840 on the list, sending 400 on Thursday.",
    ],
    ["Florian", "10:02", "Ship it. Hold back anyone who signed up today."],
];

/** The source of your apps: the path, its last change, and when. */
export const sources: readonly (readonly [name: string, change: string, when: string])[] = [
    ["@platform/waitlist", "Mail the first batch on Thursday", "2h ago"],
    ["@platform/site", "Add the waitlist section", "yesterday"],
    ["@app/tasks", "Add the launch board", "Oct 22"],
    ["destack.json", "Share Code with the agent", "Oct 18"],
    ["README.md", "Launch checklist", "Oct 14"],
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
    [2, 11, 12, "Demo · DST-14", false],
    [2, 13, 14, "Bookable", true],
    [3, 9, 10, "Invites · 400", false],
    [3, 16, 17, "Bookable", true],
    [4, 10, 10.5, "Launch", false],
    [4, 11, 12, "Bookable", true],
];

/** A signup's state, invited or still waiting. */
export type SignupState = "invited" | "waiting";

/** The latest signups: the address. */
export const signups: readonly (readonly [
    email: string,
    source: string,
    joined: string,
    state: SignupState,
])[] = [
    ["maya@northwind.io", "Launch post", "2 min ago", "waiting"],
    ["jonas@kettle.dev", "Website", "14 min ago", "waiting"],
    ["tara@fieldnotes.co", "Friend", "1 h ago", "waiting"],
    ["leo@brightpath.app", "Website", "3 h ago", "waiting"],
    ["sam@quietly.so", "Launch post", "Yesterday", "invited"],
    ["noor@papertrail.fm", "Demo day", "Yesterday", "invited"],
    ["ines@orbitlabs.eu", "Website", "Oct 18", "invited"],
];

/** The mail in the inbox, newest first. */
export const mails: readonly (readonly [
    sender: string,
    tint: string,
    subject: string,
    preview: string,
    time: string,
    isUnread: boolean,
])[] = [
    [
        "Maya Chen",
        "#a0485f",
        "Re: Destack demo",
        "Wednesday at 11:00 works, see you then",
        "10:14",
        true,
    ],
    [
        "Jonas Weber",
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
        "Maya Chen booked Wed 22 Oct, 11:00 to 11:30",
        "Oct 20",
        false,
    ],
    [
        "Replit",
        "#f26207",
        "Your export is ready",
        "waitlist-signups.csv, 1,840 rows",
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
