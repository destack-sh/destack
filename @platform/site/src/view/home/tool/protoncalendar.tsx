import * as style from "@destack/style";

import { month, week } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { protonMail } from "./protonmail.stylex";
import { protonCalendar } from "./protoncalendar.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    apps: ["M4 4h6v6H4z", "M14 4h6v6h-6z", "M4 14h6v6H4z", "M14 14h6v6h-6z"],
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
    caret: ["m7 10 5 5 5-5"],
    plus: ["M12 5v14", "M5 12h14"],
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
    check: ["m6 12 4 4 8-9"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
} satisfies Record<string, readonly string[]>;

/** The days the week shows, from Monday the 20th, Proton's weeks starting on Monday. */
const days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((day, index) => ({
    day,
    date: 20 + index,
    isWeekend: index > 4,
    isToday: 20 + index === month.today,
}));

/** The month's dates, marking the week shown and today. */
const dates = Array.from({ length: month.days }, (_, index) => ({
    date: index + 1,
    isWeek: index + 1 >= 20 && index + 1 <= 26,
    isToday: index + 1 === month.today,
}));

/** The week's events, the launch's in their own calendar; Proton offers no bookable times. */
const events = week
    .filter(([, , , , isBookable]) => !isBookable)
    .map(([day, start, end, title]) => ({
        day,
        start,
        end,
        title,
        isLaunch: title.includes(" · "),
        isShort: end - start < 1,
    }));

/** The hours the week shows, from nine to five. */
const hours: readonly number[] = [9, 10, 11, 12, 13, 14, 15, 16];

/** The days before the 1st in the month's first row: October 2025 starts on a Wednesday. */
const LEADING_DAYS = (month.first + 6) % 7;

/** The calendars in the side list: name, colour and whether they sit under other calendars. */
const calendars: readonly (readonly [name: string, tint: string, isOther: boolean])[] = [
    ["Florian", protonCalendar.mine, false],
    ["Destack launch", protonCalendar.launch, false],
    ["Swiss public holidays", protonCalendar.holidays, true],
];

/** Return the grid row an hour starts on, two rows to the hour after the day headers. */
function rowOf(hour: number): number {
    return Math.round((hour - 9) * 2) + 2;
}

/** Return a time of day as the week writes it, such as 09:30. */
function clockOf(hour: number): string {
    const minutes = Math.round((hour % 1) * 60);

    return `${String(Math.floor(hour)).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

/** Draw the launch week in the private calendar, the launch's events in their own calendar. */
export function ProtonCalendar() {
    return (
        <div data-component="ProtonCalendarPage" {...style.attrs(styles.page)}>
            {/* sidebar */}
            <nav {...style.attrs(styles.side)}>
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="protoncalendar" tint={protonMail.purple} size={22} />
                    <b>
                        Proton <span {...style.attrs(styles.brandName)}>Calendar</span>
                    </b>
                    <span {...style.attrs(styles.brandApps)}>
                        <Mark name="apps" size={16} />
                    </span>
                </span>
                <span {...style.attrs(styles.newEvent)}>New event</span>

                {/* the month */}
                <span {...style.attrs(styles.monthHead)}>
                    <b>October 2025</b>
                    <Mark name="back" size={14} />
                    <Mark name="forward" size={14} />
                </span>
                <span {...style.attrs(styles.month)}>
                    {["M", "T", "W", "T", "F", "S", "S"].map((letter) => (
                        <span {...style.attrs(styles.weekday)}>{letter}</span>
                    ))}
                    {Array.from({ length: LEADING_DAYS }, () => (
                        <span />
                    ))}
                    {dates.map((date) => (
                        <span
                            {...style.attrs(
                                styles.date,
                                date.isWeek && styles.dateWeek,
                                date.isToday && styles.dateToday,
                            )}
                        >
                            {date.date}
                        </span>
                    ))}
                </span>

                {/* the calendars */}
                <span {...style.attrs(styles.group)}>
                    <Mark name="caret" size={12} />
                    My calendars
                    <span {...style.attrs(styles.groupTools)}>
                        <Mark name="plus" size={14} />
                        <Mark name="gear" size={14} />
                    </span>
                </span>
                {calendars
                    .filter(([, , isOther]) => !isOther)
                    .map(([name, tint]) => (
                        <Calendar name={name} tint={tint} />
                    ))}
                <span {...style.attrs(styles.group)}>
                    <Mark name="caret" size={12} />
                    Other calendars
                </span>
                {calendars
                    .filter(([, , isOther]) => isOther)
                    .map(([name, tint]) => (
                        <Calendar name={name} tint={tint} />
                    ))}
            </nav>

            <div {...style.attrs(styles.main)}>
                {/* toolbar */}
                <header {...style.attrs(styles.toolbar)}>
                    <span {...style.attrs(styles.today)}>Today</span>
                    <Mark name="back" size={16} />
                    <Mark name="forward" size={16} />
                    <b {...style.attrs(styles.range)}>October 2025</b>
                    <span {...style.attrs(styles.toolbarEnd)}>
                        <Mark name="search" size={16} />
                        <span {...style.attrs(styles.view)}>
                            Week
                            <Mark name="caret" size={12} />
                        </span>
                        <span {...style.attrs(styles.me)}>F</span>
                    </span>
                </header>

                {/* week */}
                <div {...style.attrs(styles.week)}>
                    <span {...style.attrs(styles.zone)}>GMT+2</span>
                    {days.map((day, index) => (
                        <span
                            style={{ "grid-column": String(index + 2) }}
                            {...style.attrs(
                                styles.dayHead,
                                day.isWeekend && styles.weekend,
                                day.isToday && styles.dayToday,
                            )}
                        >
                            {day.day} <b>{day.date}</b>
                        </span>
                    ))}
                    {hours.map((hour) => (
                        <span
                            style={{ "grid-row": `${rowOf(hour)} / span 2` }}
                            {...style.attrs(styles.hour)}
                        >
                            {clockOf(hour)}
                        </span>
                    ))}
                    {events.map((event) => (
                        <span
                            style={{
                                "grid-column": String(event.day + 2),
                                "grid-row": `${rowOf(event.start)} / ${rowOf(event.end)}`,
                            }}
                            {...style.attrs(styles.event, event.isLaunch && styles.eventLaunch)}
                        >
                            <b {...style.attrs(styles.eventTitle)}>
                                {event.isShort
                                    ? `${event.title}, ${clockOf(event.start)}`
                                    : event.title}
                            </b>
                            {event.isShort ? undefined : (
                                <span>
                                    {clockOf(event.start)} – {clockOf(event.end)}
                                </span>
                            )}
                        </span>
                    ))}
                </div>
            </div>
        </div>
    );
}

/** Draw one calendar of the side list: its checked box in the calendar's colour and its name. */
function Calendar(properties: { name: string; tint: string }) {
    return (
        <span {...style.attrs(styles.calendar)}>
            <span style={{ "background-color": properties.tint }} {...style.attrs(styles.box)}>
                <Mark name="check" size={12} />
            </span>
            {properties.name}
        </span>
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

/** The Proton Calendar page styles. */
const styles = style.create({
    page: {
        backgroundColor: protonMail.surface,
        color: protonMail.ink,
        display: "grid",
        fontFamily: protonMail.face,
        fontSize: "13px",
        gridTemplateColumns: { default: "232px minmax(0, 1fr)", [site.compact]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
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

    // sidebar
    side: {
        alignContent: "start",
        backgroundColor: protonMail.chrome,
        display: { default: "grid", [site.compact]: "none" },
        gap: "4px",
        paddingBlock: "14px",
        paddingInline: "14px",
    },
    brand: {
        alignItems: "center",
        color: protonMail.muted,
        display: "flex",
        gap: "8px",
        paddingBlockEnd: "10px",
    },
    brandApps: {
        display: "inline-flex",
        marginInlineStart: "auto",
    },
    brandName: {
        color: protonMail.purple,
    },
    newEvent: {
        backgroundColor: protonMail.purple,
        borderRadius: "8px",
        color: protonMail.onAccent,
        fontSize: "14px",
        paddingBlock: "9px",
        textAlign: "center",
    },
    monthHead: {
        alignItems: "center",
        color: protonMail.muted,
        display: "flex",
        gap: "10px",
        paddingBlockStart: "14px",
        paddingInline: "4px",
    },
    month: {
        display: "grid",
        fontSize: "12px",
        gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
        paddingBlockEnd: "10px",
        rowGap: "2px",
        textAlign: "center",
    },
    weekday: {
        color: protonMail.muted,
        fontWeight: 600,
        paddingBlock: "6px",
    },
    date: {
        borderRadius: "6px",
        paddingBlock: "4px",
    },
    dateWeek: {
        backgroundColor: protonMail.selected,
        borderRadius: 0,
    },
    dateToday: {
        backgroundColor: protonCalendar.today,
        borderRadius: "6px",
        color: protonMail.onAccent,
        fontWeight: 700,
    },
    group: {
        alignItems: "center",
        color: protonMail.muted,
        display: "flex",
        gap: "8px",
        paddingBlock: "8px",
    },
    groupTools: {
        display: "flex",
        gap: "10px",
        marginInlineStart: "auto",
    },
    calendar: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        paddingBlock: "5px",
        paddingInlineStart: "4px",
    },
    box: {
        alignItems: "center",
        borderRadius: "4px",
        color: protonMail.onAccent,
        display: "inline-flex",
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },

    // toolbar
    main: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },
    toolbar: {
        alignItems: "center",
        borderBottomColor: protonMail.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: protonMail.muted,
        display: "flex",
        gap: "14px",
        height: "56px",
        paddingInline: "16px",
        whiteSpace: "nowrap",
    },
    today: {
        borderColor: protonMail.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: protonMail.ink,
        paddingBlock: "6px",
        paddingInline: "14px",
    },
    range: {
        color: protonMail.ink,
        fontSize: "18px",
    },
    toolbarEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    view: {
        alignItems: "center",
        borderColor: protonMail.line,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: protonMail.ink,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "6px",
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "8px",
        color: protonMail.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "30px",
        justifyContent: "center",
        width: "30px",
    },

    // week
    week: {
        backgroundImage: `linear-gradient(${protonMail.line} 1px, transparent 1px)`,
        backgroundPositionY: "44px",
        backgroundSize: "100% 44px",
        columnGap: "1px",
        display: "grid",
        gridAutoRows: "22px",
        gridTemplateColumns: {
            default: "56px repeat(7, minmax(0, 1fr))",
            [site.narrow]: "44px repeat(7, minmax(0, 1fr))",
        },
        gridTemplateRows: "44px",
        overflow: "hidden",
    },
    zone: {
        alignSelf: "end",
        color: protonMail.muted,
        fontSize: "10px",
        gridColumn: "1",
        gridRow: "1",
        paddingBlockEnd: "4px",
        textAlign: "center",
    },
    dayHead: {
        alignSelf: "center",
        color: protonMail.muted,
        fontSize: "13px",
        gridRow: "1",
        overflow: "hidden",
        textAlign: "center",
        whiteSpace: "nowrap",
    },
    weekend: {
        opacity: 0.7,
    },
    dayToday: {
        color: protonCalendar.today,
    },
    hour: {
        color: protonMail.muted,
        fontSize: "11px",
        gridColumn: "1",
        paddingInlineEnd: "8px",
        textAlign: "end",
    },
    event: {
        alignContent: "start",
        backgroundColor: protonCalendar.event,
        borderInlineStartColor: protonCalendar.mine,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "3px",
        borderRadius: "4px",
        display: "grid",
        fontSize: "11px",
        lineHeight: 1.3,
        marginInlineEnd: "3px",
        overflow: "hidden",
        paddingBlock: "3px",
        paddingInline: "6px",
    },
    eventLaunch: {
        borderInlineStartColor: protonCalendar.launch,
    },
    eventTitle: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
});
