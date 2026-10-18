import * as style from "@destack/style";

import { week } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { outlook } from "./outlook.stylex";
import { outlookCalendar } from "./outlookcalendar.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    launcher: [
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
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    mail: ["M3 6h18v12H3z", "m3 7 9 6 9-6"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    people: [
        "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
        "M3 20a6 6 0 0 1 12 0",
        "M16 11a3 3 0 1 0 0-6",
        "M17.5 14.5A5 5 0 0 1 21 19.5",
    ],
    files: ["M7 3h7l5 5v13H7z", "M14 3v5h5"],
    todo: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m8 12 3 3 5-6"],
    plus: ["M12 5v14", "M5 12h14"],
    caret: ["m7 10 5 5 5-5"],
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
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
} satisfies Record<string, readonly string[]>;

/** The rail's apps, Calendar chosen. */
const apps: readonly (keyof typeof glyphs)[] = ["mail", "calendar", "people", "files", "todo"];

/** The hours the week shows, from nine to five. */
const hours: readonly number[] = [9, 10, 11, 12, 13, 14, 15, 16];

/** The days the work week shows. */
const days: readonly string[] = [
    "20 Monday",
    "21 Tuesday",
    "22 Wednesday",
    "23 Thursday",
    "24 Friday",
];

/** Return the grid row an hour starts on, two rows to the hour after the day headers. */
function rowOf(hour: number): number {
    return Math.round((hour - 9) * 2) + 2;
}

/** Draw the launch week in the office calendar, its bookable times offered through bookings. */
export function OutlookCalendar() {
    return (
        <div data-component="OutlookCalendarPage" {...style.attrs(styles.page)}>
            {/* header bar */}
            <header {...style.attrs(styles.header)}>
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="outlook" tint="#0f6cbd" size={18} />
                    Outlook
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={16} />
                    Search
                </span>
                <span {...style.attrs(styles.headerEnd)}>
                    <Mark name="bell" size={18} />
                    <Mark name="gear" size={18} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* app rail */}
                <nav {...style.attrs(styles.rail)}>
                    {apps.map((app) => (
                        <span {...style.attrs(styles.app, app === "calendar" && styles.appOn)}>
                            <Mark name={app} size={20} />
                        </span>
                    ))}
                </nav>

                <div {...style.attrs(styles.work)}>
                    {/* ribbon */}
                    <div {...style.attrs(styles.ribbon)}>
                        <span {...style.attrs(styles.newEvent)}>
                            <Mark name="calendar" size={16} />
                            New event
                            <span {...style.attrs(styles.split)}>
                                <Mark name="caret" size={12} />
                            </span>
                        </span>
                        <span {...style.attrs(styles.command, styles.wide)}>
                            Work week
                            <Mark name="caret" size={12} />
                        </span>
                        <span {...style.attrs(styles.command)}>Today</span>
                        <Mark name="back" size={16} />
                        <Mark name="forward" size={16} />
                        <b {...style.attrs(styles.range)}>
                            <span {...style.attrs(styles.long)}>October 20–24, 2025</span>
                            <span {...style.attrs(styles.short)}>Oct 20–24</span>
                        </b>
                    </div>

                    <div {...style.attrs(styles.panes)}>
                        {/* calendars */}
                        <nav {...style.attrs(styles.side)}>
                            <span {...style.attrs(styles.group)}>
                                <Mark name="caret" size={12} />
                                My calendars
                            </span>
                            <span {...style.attrs(styles.calendar)}>
                                <span {...style.attrs(styles.box)} />
                                Calendar
                            </span>
                            <span {...style.attrs(styles.calendar)}>
                                <span {...style.attrs(styles.box, styles.boxQuiet)} />
                                Birthdays
                            </span>
                            <span {...style.attrs(styles.group)}>
                                <Mark name="caret" size={12} />
                                Bookings with me
                            </span>
                            <span {...style.attrs(styles.calendar)}>
                                <span {...style.attrs(styles.box, styles.boxBooking)} />
                                Destack demo · 30 min
                            </span>
                        </nav>

                        {/* week */}
                        <div {...style.attrs(styles.week)}>
                            {days.map((day, index) => (
                                <span
                                    style={{ "grid-column": String(index + 2) }}
                                    {...style.attrs(styles.dayHead, index === 0 && styles.today)}
                                >
                                    {day}
                                </span>
                            ))}
                            {hours.map((hour) => (
                                <span
                                    style={{ "grid-row": `${rowOf(hour)} / span 2` }}
                                    {...style.attrs(styles.hour)}
                                >
                                    {hour}:00
                                </span>
                            ))}
                            {week.map(([day, start, end, title, isBookable]) => (
                                <span
                                    style={{
                                        "grid-column": String(day + 2),
                                        "grid-row": `${rowOf(start)} / ${rowOf(end)}`,
                                    }}
                                    {...style.attrs(styles.event, isBookable && styles.booking)}
                                >
                                    <b>{isBookable ? "Destack demo" : title.split(" · ")[0]}</b>
                                    <span>{isBookable ? "Bookings with me" : "Florian Cäsar"}</span>
                                </span>
                            ))}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
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

/** The Outlook Calendar page styles. */
const styles = style.create({
    page: {
        backgroundColor: outlook.frame,
        color: outlook.ink,
        display: "grid",
        fontFamily: outlook.face,
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
        strokeWidth: 1.6,
    },

    // header bar
    header: {
        alignItems: "center",
        backgroundColor: outlook.header,
        color: outlook.muted,
        display: "flex",
        gap: "16px",
        height: "48px",
        paddingInline: "14px",
    },
    brand: {
        alignItems: "center",
        color: outlook.ink,
        display: "flex",
        fontSize: "14px",
        fontWeight: 600,
        gap: "8px",
    },
    search: {
        alignItems: "center",
        backgroundColor: outlook.search,
        borderColor: outlook.searchLine,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: outlook.muted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        height: "32px",
        marginInline: "auto",
        maxWidth: "468px",
        paddingInline: "10px",
        width: "50%",
    },
    headerEnd: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: outlook.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "48px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },
    rail: {
        alignItems: "center",
        color: outlook.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "4px",
        paddingBlockStart: "8px",
    },
    app: {
        alignItems: "center",
        borderRadius: "4px",
        display: "inline-flex",
        height: "40px",
        justifyContent: "center",
        position: "relative",
        width: "40px",
    },
    appOn: {
        color: outlook.blue,
        "::before": {
            backgroundColor: outlook.blue,
            borderRadius: "2px",
            content: '""',
            height: "20px",
            insetInlineStart: "-4px",
            position: "absolute",
            width: "3px",
        },
    },
    work: {
        display: "grid",
        gap: "8px",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        paddingBlock: { default: "8px", [site.narrow]: "0" },
        paddingInlineEnd: { default: "8px", [site.narrow]: "0" },
    },

    // ribbon
    ribbon: {
        alignItems: "center",
        backgroundColor: outlook.surface,
        borderRadius: "6px",
        display: "flex",
        gap: "12px",
        overflow: "hidden",
        paddingBlock: "8px",
        paddingInline: "10px",
        whiteSpace: "nowrap",
    },
    newEvent: {
        alignItems: "center",
        backgroundColor: outlook.blue,
        borderRadius: "4px",
        color: outlook.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "6px",
        height: "32px",
        paddingInlineStart: "10px",
    },
    split: {
        alignItems: "center",
        borderInlineStartColor: outlook.split,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "inline-flex",
        height: "100%",
        paddingInline: "6px",
    },
    command: {
        alignItems: "center",
        borderColor: outlook.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        gap: "6px",
        height: "30px",
        paddingInline: "10px",
    },
    wide: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },
    long: {
        display: { default: "inline", [site.narrow]: "none" },
    },
    short: {
        display: { default: "none", [site.narrow]: "inline" },
    },
    range: {
        fontSize: "15px",
    },
    panes: {
        display: "grid",
        gap: "8px",
        gridTemplateColumns: { default: "180px minmax(0, 1fr)", [site.compact]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },
    side: {
        display: { default: "flex", [site.compact]: "none" },
        flexDirection: "column",
        gap: "2px",
    },
    group: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "6px",
    },
    calendar: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        paddingBlock: "5px",
        paddingInlineStart: "24px",
    },
    box: {
        backgroundColor: outlook.blue,
        borderRadius: "3px",
        height: "14px",
        width: "14px",
    },
    boxQuiet: {
        backgroundColor: outlookCalendar.birthdays,
    },
    boxBooking: {
        backgroundColor: outlookCalendar.booking,
    },

    // week
    week: {
        backgroundColor: outlook.surface,
        backgroundImage: `linear-gradient(${outlook.line} 1px, transparent 1px)`,
        backgroundPositionY: "36px",
        backgroundSize: "100% 44px",
        borderRadius: "6px",
        columnGap: "1px",
        display: "grid",
        gridAutoRows: "22px",
        gridTemplateColumns: "48px repeat(5, minmax(0, 1fr))",
        gridTemplateRows: "36px",
        overflow: "hidden",
    },
    dayHead: {
        alignSelf: "center",
        color: outlook.muted,
        fontSize: "12px",
        gridRow: "1",
        paddingInline: "6px",
        whiteSpace: "nowrap",
    },
    today: {
        color: outlook.blue,
        fontWeight: 700,
    },
    hour: {
        color: outlook.muted,
        fontSize: "11px",
        gridColumn: "1",
        paddingInlineEnd: "6px",
        textAlign: "end",
    },
    event: {
        alignContent: "start",
        backgroundColor: outlookCalendar.event,
        borderInlineStartColor: outlook.blue,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        borderRadius: "4px",
        display: "grid",
        fontSize: "11px",
        lineHeight: 1.3,
        marginInlineEnd: "4px",
        overflow: "hidden",
        paddingBlock: "3px",
        paddingInline: "6px",
    },
    booking: {
        backgroundColor: outlookCalendar.bookingFill,
        borderInlineStartColor: outlookCalendar.booking,
    },
});
