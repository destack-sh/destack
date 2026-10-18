import * as style from "@destack/style";

import { week, type WeekBlock } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { googleCalendar } from "./googlecalendar.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    help: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6",
        "M12 17h.01",
    ],
    gear: [
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M4 12h2",
        "M18 12h2",
        "M12 4v2",
        "M12 18v2",
        "m6.3 6.3 1.4 1.4",
        "m16.3 16.3 1.4 1.4",
        "m6.3 17.7 1.4-1.4",
        "m16.3 7.7 1.4-1.4",
    ],
    caret: ["m7 10 5 5 5-5"],
    apps: [
        "M5 5h.01",
        "M12 5h.01",
        "M19 5h.01",
        "M5 12h.01",
        "M12 12h.01",
        "M19 12h.01",
        "M5 19h.01",
        "M12 19h.01",
        "M19 19h.01",
    ],
    plus: ["M12 5v14", "M5 12h14"],
    booking: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4", "m9 15 2 2 4-4"],
} satisfies Record<string, readonly string[]>;

/** The hours the week shows, from nine to five. */
const hours: readonly number[] = [9, 10, 11, 12, 13, 14, 15, 16];

/** The days the week shows: weekday and date. */
const days: readonly (readonly [weekday: string, date: number])[] = [
    ["MON", 20],
    ["TUE", 21],
    ["WED", 22],
    ["THU", 23],
    ["FRI", 24],
];

/** The calendars in the side list: name and colour. */
const calendars: readonly (readonly [name: string, tint: string])[] = [
    ["Florian Cäsar", "#039be5"],
    ["Destack launch", "#33b679"],
    ["Tasks", "#4285f4"],
];

/** Return the grid row an hour starts on, two rows to the hour after the day headers. */
function rowOf(hour: number): number {
    return Math.round((hour - 9) * 2) + 2;
}

/** Draw the launch week in the calendar as it stands in the launch story. */
export function GoogleCalendar() {
    return <GoogleCalendarWeek week={week} />;
}

/** Draw a week in the calendar: bookable times as a schedule, overlaps side by side. */
export function GoogleCalendarWeek(properties: { week: readonly WeekBlock[] }) {
    return (
        <div data-component="GoogleCalendarPage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.top)}>
                <Mark name="menu" size={22} />
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="googlecalendar" tint="#4285f4" size={30} />
                    Calendar
                </span>
                <span {...style.attrs(styles.today)}>Today</span>
                <Mark name="back" size={18} />
                <Mark name="forward" size={18} />
                <b {...style.attrs(styles.month)}>October 2025</b>
                <span {...style.attrs(styles.topEnd)}>
                    <Mark name="search" size={20} />
                    <Mark name="help" size={20} />
                    <Mark name="gear" size={20} />
                    <span {...style.attrs(styles.view)}>
                        Week
                        <Mark name="caret" size={14} />
                    </span>
                    <Mark name="apps" size={20} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* side */}
                <nav {...style.attrs(styles.side)}>
                    <span {...style.attrs(styles.create)}>
                        <Mark name="plus" size={22} />
                        Create
                        <Mark name="caret" size={14} />
                    </span>
                    <span {...style.attrs(styles.schedule)}>
                        <Mark name="booking" size={16} />
                        Booking pages
                    </span>
                    <span {...style.attrs(styles.scheduleItem)}>Destack demo · 30 min</span>
                    <span {...style.attrs(styles.group)}>My calendars</span>
                    {calendars.map(([name, tint]) => (
                        <span {...style.attrs(styles.calendar)}>
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.box)}
                            />
                            {name}
                        </span>
                    ))}
                </nav>

                {/* week */}
                <div {...style.attrs(styles.week)}>
                    {days.map(([weekday, date], index) => (
                        <span
                            style={{ "grid-column": String(index + 2) }}
                            {...style.attrs(styles.dayHead)}
                        >
                            <span {...style.attrs(styles.weekday, index === 0 && styles.todayText)}>
                                {weekday}
                            </span>
                            <span {...style.attrs(styles.date, index === 0 && styles.dateToday)}>
                                {date}
                            </span>
                        </span>
                    ))}
                    {hours.map((hour) => (
                        <span
                            style={{ "grid-row": `${rowOf(hour)} / span 2` }}
                            {...style.attrs(styles.hour)}
                        >
                            {hour < 12 ? `${hour} AM` : hour === 12 ? "12 PM" : `${hour - 12} PM`}
                        </span>
                    ))}
                    {properties.week.map(([day, start, end, title, isBookable], index) => (
                        <span
                            style={{
                                "grid-column": String(day + 2),
                                "grid-row": `${rowOf(start)} / ${rowOf(end)}`,
                            }}
                            {...style.attrs(
                                styles.event,
                                isBookable
                                    ? styles.slot
                                    : title.includes("·")
                                      ? styles.sage
                                      : styles.own,
                                overlapOf(properties.week, index) === "first" && styles.eventFirst,
                                overlapOf(properties.week, index) === "second" &&
                                    styles.eventSecond,
                            )}
                        >
                            <b>{isBookable ? "Destack demo" : title.split(" · ")[0]}</b>
                            <span>
                                {isBookable
                                    ? "Open for booking"
                                    : `${Math.floor(start)}:${start % 1 === 0 ? "00" : "30"}`}
                            </span>
                        </span>
                    ))}
                </div>
            </div>
        </div>
    );
}

/** Return whether a block shares its start with another: the first or second of two, or alone. */
function overlapOf(blocks: readonly WeekBlock[], index: number): "first" | "second" | "alone" {
    // find the blocks that start when this one does on the same day
    const block = blocks[index];
    const shared = blocks.filter((other) => other[0] === block?.[0] && other[1] === block?.[1]);
    if (shared.length < 2) {
        return "alone";
    }

    return shared[0] === block ? "first" : "second";
}

/** Draw one of the app's own glyphs in the current colour. */
function Mark(properties: { name: keyof typeof glyphs; size: number }) {
    return (
        <svg
            aria-hidden="true"
            viewBox="0 0 24 24"
            style={{ height: `${properties.size}px`, width: `${properties.size}px` }}
            {...style.attrs(styles.mark)}
        >
            {glyphs[properties.name].map((path) => (
                <path d={path} />
            ))}
        </svg>
    );
}

/** The Google Calendar page styles. */
const styles = style.create({
    page: {
        backgroundColor: googleCalendar.chrome,
        color: googleCalendar.ink,
        display: "grid",
        fontFamily: googleCalendar.face,
        fontSize: "13px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },

    // top bar
    top: {
        alignItems: "center",
        color: googleCalendar.muted,
        display: "flex",
        gap: "14px",
        height: "64px",
        paddingInline: "16px",
    },
    brand: {
        alignItems: "center",
        color: googleCalendar.muted,
        display: "flex",
        fontSize: "22px",
        gap: "8px",
        marginInlineEnd: "16px",
    },
    today: {
        borderColor: googleCalendar.muted,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleCalendar.ink,
        display: { default: "inline", [site.narrow]: "none" },
        fontWeight: 500,
        paddingBlock: "8px",
        paddingInline: "18px",
    },
    month: {
        color: googleCalendar.ink,
        fontSize: "20px",
        fontWeight: 400,
        whiteSpace: "nowrap",
    },
    topEnd: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    view: {
        alignItems: "center",
        borderColor: googleCalendar.muted,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleCalendar.ink,
        display: { default: "inline-flex", [site.compact]: "none" },
        fontWeight: 500,
        gap: "6px",
        paddingBlock: "7px",
        paddingInline: "14px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: googleCalendar.onAccent,
        display: "inline-flex",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "230px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // side
    side: {
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "4px",
        paddingInline: "12px",
    },
    create: {
        alignItems: "center",
        backgroundColor: googleCalendar.create,
        borderRadius: "16px",
        boxShadow: googleCalendar.lift,
        display: "inline-flex",
        fontWeight: 500,
        gap: "12px",
        height: "56px",
        marginBlockEnd: "18px",
        paddingInlineEnd: "20px",
        paddingInlineStart: "16px",
        width: "fit-content",
    },
    schedule: {
        alignItems: "center",
        display: "flex",
        fontWeight: 500,
        gap: "10px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    scheduleItem: {
        color: googleCalendar.muted,
        paddingBlockEnd: "12px",
        paddingInlineStart: "34px",
    },
    group: {
        fontWeight: 500,
        paddingBlock: "8px",
        paddingInline: "8px",
    },
    calendar: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    box: {
        borderRadius: "3px",
        height: "16px",
        width: "16px",
    },

    // week
    week: {
        backgroundColor: googleCalendar.surface,
        backgroundImage: `linear-gradient(${googleCalendar.line} 1px, transparent 1px)`,
        backgroundPositionY: "56px",
        backgroundSize: "100% 44px",
        borderStartStartRadius: { default: "24px", [site.narrow]: "0" },
        columnGap: "1px",
        display: "grid",
        gridAutoRows: "22px",
        gridTemplateColumns: "56px repeat(5, minmax(0, 1fr))",
        gridTemplateRows: "56px",
        overflow: "hidden",
        paddingInlineEnd: "8px",
    },
    dayHead: {
        alignItems: "center",
        display: "grid",
        gridRow: "1",
        justifyItems: "center",
    },
    weekday: {
        color: googleCalendar.muted,
        fontSize: "11px",
        fontWeight: 500,
    },
    todayText: {
        color: googleCalendar.blue,
    },
    date: {
        alignItems: "center",
        borderRadius: "50%",
        display: "inline-flex",
        fontSize: "22px",
        height: "40px",
        justifyContent: "center",
        width: "40px",
    },
    dateToday: {
        backgroundColor: googleCalendar.blue,
        color: googleCalendar.surface,
    },
    hour: {
        color: googleCalendar.muted,
        fontSize: "10px",
        gridColumn: "1",
        paddingInlineEnd: "8px",
        textAlign: "end",
        transform: "translateY(-6px)",
    },
    event: {
        borderRadius: "6px",
        display: "grid",
        alignContent: "start",
        fontSize: "11px",
        lineHeight: 1.3,
        marginInlineEnd: "6px",
        overflow: "hidden",
        paddingBlock: "3px",
        paddingInline: "6px",
    },
    own: {
        backgroundColor: googleCalendar.event,
        color: googleCalendar.onAccent,
    },
    sage: {
        backgroundColor: googleCalendar.sage,
        color: googleCalendar.onAccent,
    },
    slot: {
        backgroundColor: googleCalendar.slot,
        borderColor: googleCalendar.slotEdge,
        borderStyle: "dashed",
        borderWidth: "1px",
        color: googleCalendar.blue,
    },
    eventFirst: {
        marginInlineEnd: "50%",
    },
    eventSecond: {
        marginInlineStart: "50%",
    },
});
