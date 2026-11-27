import type { Hue } from "@destack/style";
import { PlainDate, PlainTime, TimeZone } from "@destack/schema";
import type { CalendarEvent } from "@destack/ui/event-calendar";
import { type Accessor, createContext, useContext } from "@destack/view";

/** The language the products in the story write their dates in, the site's own. */
const TAG = "en-US";

/** A date as most of the story's products write it, such as Oct 22. */
const SHORT: Intl.DateTimeFormatOptions = { month: "short", day: "numeric" };

/** The time the story stands at on its Monday, after the morning's mail and messages. */
const NOW = PlainTime.parse("10:36");

/** The days the books trail the week, so early in a month the receipts are the month before's. */
const BOOKS_LAG = 10;

/** The day of the booking a client chose on the booking page: the Tuesday after next. */
export const CHOSEN_DAY = 8;

/** The day phase 2 hands over to Ines: a Friday, five weeks on. */
export const HANDOVER_DAY = 39;

/** The Monday page's overview: a short title and its line. */
export const overview: readonly (readonly [title: string, line: string])[] = [
    ["Promised", "3 things you said you'd do"],
    ["Due", "4 tasks before Friday"],
    ["Free", "6 hours to do them"],
];

/** The faces of the people waiting on a reply from you: an initial and a tint. */
export const owed: readonly (readonly [initial: string, tint: Hue])[] = [
    ["I", "pink"],
    ["J", "blue"],
    ["M", "orange"],
];

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

/** A task: its key, title, state, owner, owner's colour, and the day it is due. */
export type Task = readonly [
    id: string,
    title: string,
    state: TaskState,
    owner: string,
    tint: Hue,
    due: number,
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

/** A block of your week: its day, start and end hours, title, and whether it is bookable. */
export type WeekBlock = readonly [
    day: number,
    start: number,
    end: number,
    title: string,
    isBookable: boolean,
];

/** Your week, Monday to Friday. */
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

/** A commitment: to whom, what you said, where and on which day, its state, and a note. */
export type Commitment = readonly [
    person: string,
    what: string,
    channel: string,
    said: number,
    state: CommitmentState,
    note: string,
];

/** A mail: its sender, their colour, subject, preview, when it came, and whether it is unread. */
export type Mail = readonly [
    sender: string,
    tint: string,
    subject: string,
    preview: string,
    time: string,
    isUnread: boolean,
];

/** Whose turn it is with a person: yours to reply, theirs, or nobody's. */
export type ReplyState = "owe" | "waiting" | "clear";

/** A person you deal with: name, relation, whose turn it is, the last mail's day, and a tint. */
export type Person = readonly [
    name: string,
    relation: string,
    state: ReplyState,
    last: number,
    tint: Hue,
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

/** A workflow: its name, what starts it, its runs this month, and its last run. */
export type Workflow = readonly [name: string, trigger: string, runs: string, last: string];

/** A run of a workflow: the workflow, its subject, its steps, its duration and when. */
export type Run = readonly [
    workflow: string,
    subject: string,
    steps: number,
    duration: string,
    when: string,
];

/** The month the booking page shows: its length, its first weekday from Sunday, and its days. */
export interface BookingMonth {
    /** The first day of the month. */
    readonly first: PlainDate;
    /** The days in the month. */
    readonly days: number;
    /** The column of its first day, from 0 for Sunday. */
    readonly offset: number;
    /** The days of the month with free times. */
    readonly open: readonly number[];
    /** Today's day of the month, or null in another month. */
    readonly today: number | null;
    /** The day of the month a client chose. */
    readonly chosen: number;
}

/** The week the home page's apps tell, from its Monday morning, with every date in it. */
export class Story {
    /** The Monday the story plays on. */
    readonly monday: PlainDate;
    /** The time zone the story's times run in. */
    readonly zone: string;
    /** The instant the story stands at: the Monday at 10:36. */
    readonly now: number;

    /** Your tasks, yours and your agent's. */
    readonly tasks: readonly Task[];
    /** What you said you'd do, in mail and chat, newest first. */
    readonly commitments: readonly Commitment[];
    /** The mail in the inbox, newest first. */
    readonly mails: readonly Mail[];
    /** The Receipts label: the mail your agent's filter moved there, newest first. */
    readonly receipts: readonly Mail[];
    /** All Mail: the newsletters your agent archived, Kai's sketch swept up among them. */
    readonly archived: readonly Mail[];
    /** The people you deal with. */
    readonly people: readonly Person[];
    /** Your space's workflows. */
    readonly workflows: readonly Workflow[];
    /** Your workflows' latest runs. */
    readonly runs: readonly Run[];
    /** Your week as calendar events, the free times your booking page offers in the background. */
    readonly events: readonly CalendarEvent[];
    /** The month the booking page shows. */
    readonly month: BookingMonth;

    /** Tell the story in the week of a Monday, its times in a time zone. */
    constructor(monday: PlainDate, zone: string) {
        // stand at the Monday morning
        this.monday = monday;
        this.zone = zone;
        this.now = this.at(0, NOW);

        // date the tasks within the week, the receipts and invoices in the month the books close
        const month = this.monthName(-BOOKS_LAG);
        this.tasks = [
            ["T-12", "Ship the booking fix", "done", "F", "cyan", 0],
            ["T-14", "Send Ines the estimate", "started", "F", "cyan", 2],
            ["T-16", `Match ${month}'s receipts`, "review", "A", "purple", 3],
            ["T-13", `Invoice ${month}`, "todo", "F", "cyan", 4],
            ["T-15", "Draft the reply to Jonas", "todo", "A", "purple", 1],
            ["T-17", "Renew my passport", "todo", "F", "cyan", 3],
            ["T-18", "Find out what the £19 charge is", "todo", "A", "purple", 4],
            ["T-19", "Cancel the subscriptions I don't use", "todo", "F", "cyan", 7],
        ];
        this.commitments = [
            ["Ines Ferreira", "Send the phase 2 estimate", "Mail", -3, "open", ""],
            ["Jonas Weber", "Tell him about Saturday", "Chat", -2, "open", ""],
            ["Mia Sørensen", `Send ${month}'s receipts`, "Mail", -6, "open", ""],
            ["Kai Brandt", "Look at his onboarding sketch", "Mail", -10, "waiting", ""],
            ["Ines Ferreira", "Fix the double booking", "Call", -7, "kept", ""],
            ["Lena Okoye", "Send the handover notes", "Mail", -11, "kept", ""],
            ["Mia Sørensen", "Sign the engagement letter", "Mail", -18, "kept", ""],
        ];

        // date the inbox: this morning's mail by its time, older mail by its day
        const ticket = this.label(5, { weekday: "long", day: "numeric", month: "long" }, "en-GB");
        this.mails = [
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
                `${month} receipts`,
                "Could you send them before Friday? I'll file the return next week.",
                "08:52",
                false,
            ],
            [
                "GitHub",
                "#24292f",
                "[booking] Fix double booking (#48)",
                "Merged into main by florian",
                this.label(0),
                false,
            ],
            [
                "Stripe",
                "#635bff",
                "Your payout is on its way",
                `It arrives in your account on ${this.label(1)}`,
                this.label(0),
                false,
            ],
            [
                "Figma",
                "#a259ff",
                "Your receipt from Figma",
                "Professional plan, 1 editor, monthly",
                this.label(-1),
                false,
            ],
            [
                "Kai Brandt",
                "#5a7a2e",
                "Onboarding sketch",
                "No rush, but I'd love your take before I build it.",
                this.label(-1),
                false,
            ],
            [
                "Hetzner",
                "#d50c2d",
                `Invoice for ${this.monthName(0, -1)}`,
                "Your invoice is ready to download",
                this.label(-3),
                false,
            ],
            [
                "Vercel",
                "#171717",
                "Failed production deployment",
                "booking: the build failed on main",
                this.label(-3),
                false,
            ],
            [
                "Notion",
                "#191919",
                "Your receipt from Notion",
                "Plus plan, monthly",
                this.label(-4),
                false,
            ],
            [
                "Trainline",
                "#01c3a7",
                "Your e-ticket",
                `Return, ${ticket}, 08:04`,
                this.label(-5),
                false,
            ],
        ];
        const first = this.label(this.firstOfMonth());
        this.receipts = [
            [
                "Figma",
                "#a259ff",
                "Your receipt from Figma",
                "Professional plan, 1 editor",
                this.label(-1),
                false,
            ],
            [
                "Hetzner",
                "#d50c2d",
                `Invoice for ${this.monthName(0, -1)}`,
                "Your invoice is ready",
                this.label(-3),
                false,
            ],
            [
                "Notion",
                "#191919",
                "Your receipt from Notion",
                "Plus plan, monthly",
                this.label(-4),
                false,
            ],
            ["Trainline", "#01c3a7", "Your e-ticket", `Return, ${ticket}`, this.label(-5), false],
            ["Vercel", "#171717", "Your Vercel invoice", `Pro plan, ${month}`, first, false],
            ["Spotify", "#1db954", "Your Spotify receipt", "Premium Individual", first, false],
        ];
        this.archived = [
            [
                "Kai Brandt",
                "#5a7a2e",
                "Onboarding sketch",
                "No rush, but I'd love your take before I build it.",
                this.label(-1),
                false,
            ],
            [
                "Stratechery",
                "#2f7d8c",
                "The weekly article",
                "This week: agents and the aggregators",
                this.label(0),
                false,
            ],
            [
                "The Pragmatic Engineer",
                "#6b5ca5",
                "The Pulse",
                "What's new in engineering this week",
                this.label(-2),
                false,
            ],
            [
                "Product Hunt",
                "#da552f",
                "The Daily Digest",
                "Today's top launches",
                this.label(-2),
                false,
            ],
            [
                "Hacker Newsletter",
                "#ff6600",
                "Hacker Newsletter #745",
                "The best of Hacker News this week",
                this.label(-3),
                false,
            ],
            [
                "Figma",
                "#a259ff",
                "What's new in Figma",
                "Updates from Config",
                this.label(-5),
                false,
            ],
        ];
        this.people = [
            ["Ines Ferreira", "Client · booking app", "owe", 0, "pink"],
            ["Jonas Weber", "Friend", "owe", 0, "blue"],
            ["Mia Sørensen", "Accountant", "owe", 0, "orange"],
            ["Kai Brandt", "Designer", "waiting", -1, "green"],
            ["Lena Okoye", "Former client", "clear", -11, "brown"],
            ["Ravi Patel", "Climbing", "clear", -16, "purple"],
        ];

        // date the workflows' runs: last Friday's review, and the month's invoices on its first
        this.workflows = [
            ["file-receipts", "on mail.create from billing", "31", "2 min ago"],
            ["owe-reply", "daily 08:00", "20", "today"],
            ["weekly-review", "Fri 16:00", "3", this.label(-3)],
            ["invoice-month", "1st of the month", "1", first],
        ];
        this.runs = [
            ["file-receipts", "Figma · £12.00", 3, "0.8 s", "2 min ago"],
            ["owe-reply", "3 people waiting", 2, "1.4 s", "08:00"],
            ["file-receipts", "Notion · £8.00", 3, "0.7 s", this.label(-4)],
            ["file-receipts", "Hetzner · £16.40", 3, "0.9 s", this.label(-3)],
            ["weekly-review", `Week of ${this.label(-7)}`, 4, "6.2 s", this.label(-3)],
            ["invoice-month", `${this.monthName(0, -1)} · 3 clients`, 3, "2.1 s", first],
        ];

        // put the week's blocks on the calendar, the bookable ones behind the rest
        this.events = [
            {
                key: "books",
                title: "Books close Friday",
                start: monday,
                end: this.date(4),
                color: "yellow",
            },
            ...week.map(([day, start, end, title, isBookable], index): CalendarEvent => ({
                key: String(index),
                title: isBookable ? "Open slot" : title,
                start: this.at(day, wallTimeOf(start)),
                end: this.at(day, wallTimeOf(end)),
                color: isBookable ? "green" : title.includes(" · ") ? "orange" : "cyan",
                display: isBookable ? "background" : "auto",
            })),
        ];

        // lay out the chosen day's month, its free days the open ones of the next two weeks
        const chosen = this.date(CHOSEN_DAY);
        const firstDay = PlainDate.with(chosen, { day: 1 });
        const isSameMonth = (date: PlainDate) => PlainDate.month(date) === PlainDate.month(chosen);
        this.month = {
            first: firstDay,
            days: PlainDate.daysInMonth(chosen),
            offset: PlainDate.dayOfWeek(firstDay) % 7,
            open: [1, 2, 3, 7, 8, 10, 11]
                .map((day) => this.date(day))
                .filter(isSameMonth)
                .map((date) => PlainDate.day(date)),
            today: isSameMonth(monday) ? PlainDate.day(monday) : null,
            chosen: PlainDate.day(chosen),
        };
    }

    /** Tell the story in the week of a day: its own week, or the next one on a weekend. */
    static of(today: PlainDate, zone: string): Story {
        // step back to Monday on a weekday, and on to the coming Monday at the weekend
        const weekday = PlainDate.dayOfWeek(today);
        const days = weekday >= 6 ? 8 - weekday : 1 - weekday;

        return new Story(PlainDate.add(today, { days }), zone);
    }

    /** Read a day of the story, counted from its Monday, such as -3 for the Friday before. */
    date(day: number): PlainDate {
        return PlainDate.add(this.monday, { days: day });
    }

    /** Read the instant of a wall time on a day of the story. */
    at(day: number, time: PlainTime): number {
        return TimeZone.instant(this.zone, this.date(day), time);
    }

    /** Write a day of the story as a product writes it, such as Oct 22 or Tuesday 21 October. */
    label(day: number, options: Intl.DateTimeFormatOptions = SHORT, tag: string = TAG): string {
        const format = new Intl.DateTimeFormat(tag, { ...options, timeZone: "UTC" });

        return format.format(this.#midnight(day));
    }

    /** Write the span between two days of the story, such as October 20 – 24. */
    labelRange(from: number, to: number, options: Intl.DateTimeFormatOptions = SHORT): string {
        const format = new Intl.DateTimeFormat(TAG, { ...options, timeZone: "UTC" });

        return format.formatRange(this.#midnight(from), this.#midnight(to));
    }

    /** Write a day as a plan does: its weekday and date within the week, its date after it. */
    labelPlan(day: number): string {
        // name a weekday of the week by its short name and date, such as Wed 22
        if (day >= 0 && day <= 4) {
            return `${this.label(day, { weekday: "short" })} ${this.label(day, { day: "numeric" })}`;
        }

        return this.label(day);
    }

    /** Write the name of the month of a day, or of a month before or after it. */
    monthName(day: number, months = 0): string {
        const first = PlainDate.add(PlainDate.with(this.date(day), { day: 1 }), { months });

        return this.label(PlainDate.daysUntil(this.monday, first), { month: "long" });
    }

    /** Read the midnight of a day of the story in UTC, which keeps it on its own day. */
    #midnight(day: number): number {
        const date = this.date(day);

        return Date.UTC(PlainDate.year(date), PlainDate.month(date) - 1, PlainDate.day(date));
    }

    /** Read the day of the first of the Monday's month, counted from the Monday. */
    firstOfMonth(): number {
        return 1 - PlainDate.day(this.monday);
    }
}

/** The story the home page's figures tell, which moves to the reader's week once the page loads. */
export const StoryContext = createContext<Accessor<Story> | null>(null);

/** Read the story the home page tells, refusing a figure outside the home page. */
export function useStory(): Accessor<Story> {
    const story = useContext(StoryContext);
    if (story === null) {
        throw new TypeError("a home figure needs the home page's story around it");
    }

    return story;
}

/** Read an hour of the day as a wall time, such as 9.5 as 09:30. */
export function wallTimeOf(hour: number): PlainTime {
    const hours = String(Math.floor(hour)).padStart(2, "0");
    const minutes = String(Math.round((hour % 1) * 60)).padStart(2, "0");

    return PlainTime.parse(`${hours}:${minutes}`);
}
