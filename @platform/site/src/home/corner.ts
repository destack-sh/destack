/** Your posts: the title, the topic, the date, and the two tints of the cover. */
export const posts: readonly (readonly [
    title: string,
    topic: string,
    date: string,
    from: string,
    to: string,
])[] = [
    ["What we learned shipping pricing v4", "Product", "Oct 18, 2025", "#e9b384", "#c4683a"],
    ["Notes from demo day", "Events", "Oct 9, 2025", "#9fc3c9", "#3f7d88"],
    ["Why I moved my tools home", "Tools", "Sep 30, 2025", "#c9c39f", "#6f7a3f"],
];

/** What the waitlist promises: a short title and its line. */
export const perks: readonly (readonly [title: string, line: string])[] = [
    ["Early pricing", "Lock in v4 at the launch rate"],
    ["First invites", "1,840 people, in batches from Thursday"],
    ["A direct line", "Book a call with the founders"],
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

/** October 2025: its length, the weekday the first falls on from Sunday, the days with free times, today, and the chosen day. */
export const month = {
    days: 31,
    first: 3,
    open: [21, 22, 23, 27, 28, 30, 31],
    today: 20,
    chosen: 28,
};

/** The free times on the chosen day. */
export const times: readonly string[] = ["09:30", "10:00", "11:30", "15:00", "16:30"];

/** The launch plan's checklist: the item and whether it is done. */
export const launch: readonly (readonly [item: string, isDone: boolean])[] = [
    ["Freeze the pricing page", true],
    ["Record the demo", false],
    ["Email the waitlist", false],
    ["Publish the launch post", false],
];

/** A task's state, from not started to done. */
export type TaskState = "todo" | "started" | "review" | "done";

/** The launch tasks: the identifier, the title, the state, the owner's initial and tint, and the due date. */
export const tasks: readonly (readonly [
    id: string,
    title: string,
    state: TaskState,
    owner: string,
    tint: string,
    due: string,
])[] = [
    ["LCH-12", "Pricing page", "done", "F", "#2f7d8c", "Oct 20"],
    ["LCH-14", "Record the demo", "started", "F", "#2f7d8c", "Oct 23"],
    ["LCH-16", "Waitlist email", "review", "A", "#6b5ca5", "Oct 23"],
    ["LCH-13", "Turn on the new prices", "todo", "F", "#2f7d8c", "Oct 24"],
    ["LCH-15", "Draft the launch post", "todo", "A", "#6b5ca5", "Oct 24"],
    ["LCH-17", "Open signups", "todo", "F", "#2f7d8c", "Oct 24"],
];

/** The launch channel's messages: who wrote it, when, and what they said. */
export const messages: readonly (readonly [
    author: "Florian" | "Agent",
    time: string,
    text: string,
])[] = [
    ["Florian", "09:12", "Can we move the demo to Wednesday? Thursday is the waitlist send."],
    ["Agent", "09:13", "Moved it to Wed 11:00 and updated LCH-14 and the launch plan."],
    [
        "Agent",
        "09:40",
        "First invite batch is drafted. 1,840 on the list, sending 400 on Thursday.",
    ],
    ["Florian", "10:02", "Ship it. Hold back anyone who signed up today."],
];

/** The source of your apps: the path, its last change, and when. */
export const sources: readonly (readonly [name: string, change: string, when: string])[] = [
    ["apps/waitlist", "Mail the first batch on Thursday", "2h ago"],
    ["apps/site", "Add the Waitlist section", "yesterday"],
    ["apps/my-week", "Show a friend's free slots", "Oct 20"],
    ["destack.json", "Share Code with the agent", "Oct 18"],
    ["README.md", "Launch checklist", "Oct 14"],
];
