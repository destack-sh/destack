import * as style from "@destack/style";

import { month, times } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { calcom } from "./calcom.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    clock: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5l3 2"],
    video: ["m16 10 5-3v10l-5-3", "M3 6h13v12H3z"],
    globe: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M3 12h18",
        "M12 3a14 14 0 0 1 0 18a14 14 0 0 1 0-18",
    ],
    caret: ["m7 10 5 5 5-5"],
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
} satisfies Record<string, readonly string[]>;

/** The weekdays over the month's grid. */
const weekdays: readonly string[] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];

/** Draw your booking page as an open scheduling booker, with a day chosen. */
export function CalCom() {
    return (
        <div data-component="CalComPage" {...style.attrs(styles.page)}>
            <div {...style.attrs(styles.booker)}>
                {/* host and event */}
                <div {...style.attrs(styles.event)}>
                    <span {...style.attrs(styles.avatar)}>F</span>
                    <span {...style.attrs(styles.host)}>Florian</span>
                    <b {...style.attrs(styles.title)}>Destack demo</b>
                    <span {...style.attrs(styles.detail)}>
                        <Mark name="clock" size={16} />
                        30m
                    </span>
                    <span {...style.attrs(styles.detail)}>
                        <Mark name="video" size={16} />
                        Cal Video
                    </span>
                    <span {...style.attrs(styles.detail)}>
                        <Mark name="globe" size={16} />
                        Europe/Zurich
                        <Mark name="caret" size={14} />
                    </span>
                </div>

                {/* month */}
                <div {...style.attrs(styles.month)}>
                    <span {...style.attrs(styles.monthHead)}>
                        <b>
                            October <span {...style.attrs(styles.year)}>2025</span>
                        </b>
                        <span {...style.attrs(styles.steps)}>
                            <Mark name="back" size={16} />
                            <Mark name="forward" size={16} />
                        </span>
                    </span>
                    <div {...style.attrs(styles.grid)}>
                        {weekdays.map((day) => (
                            <span {...style.attrs(styles.weekday)}>{day}</span>
                        ))}
                        {Array.from({ length: month.first }, () => (
                            <span />
                        ))}
                        {Array.from({ length: month.days }, (_, index) => index + 1).map((date) => (
                            <span
                                {...style.attrs(
                                    styles.date,
                                    month.open.includes(date) && styles.dateOpen,
                                    date === month.chosen && styles.dateChosen,
                                )}
                            >
                                {date}
                                {date === month.today ? (
                                    <span {...style.attrs(styles.today)} />
                                ) : undefined}
                            </span>
                        ))}
                    </div>
                </div>

                {/* times */}
                <div {...style.attrs(styles.times)}>
                    <span {...style.attrs(styles.timesHead)}>
                        <b>
                            Tue <span {...style.attrs(styles.year)}>28</span>
                        </b>
                        <span {...style.attrs(styles.toggle)}>
                            <span>12h</span>
                            <span {...style.attrs(styles.toggleOn)}>24h</span>
                        </span>
                    </span>
                    {times.map((time) => (
                        <span {...style.attrs(styles.time)}>
                            <span {...style.attrs(styles.dot)} />
                            {time}
                        </span>
                    ))}
                </div>
            </div>
            <span {...style.attrs(styles.brand)}>Cal.com</span>
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

/** The Cal.com page styles. */
const styles = style.create({
    page: {
        alignContent: { default: "center", [site.narrow]: "start" },
        backgroundColor: calcom.page,
        color: calcom.ink,
        display: "grid",
        fontFamily: calcom.text,
        fontSize: "14px",
        gap: "16px",
        gridTemplateColumns: "minmax(0, 1fr)",
        height: "100%",
        justifyItems: "center",
        overflow: "hidden",
        padding: { default: "24px", [site.narrow]: "12px" },
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 1.8,
    },
    booker: {
        backgroundColor: calcom.card,
        borderColor: calcom.line,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gridTemplateColumns: {
            default: "200px minmax(0, 1fr) 150px",
            [site.compact]: "160px minmax(0, 1fr) 140px",
            [site.narrow]: "minmax(0, 1fr)",
        },
        maxWidth: "860px",
        width: "100%",
    },
    event: {
        alignContent: "start",
        borderBottomColor: calcom.line,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: 0, [site.narrow]: "1px" },
        borderInlineEndColor: calcom.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: { default: "1px", [site.narrow]: 0 },
        display: "grid",
        gap: { default: "10px", [site.narrow]: "6px" },
        padding: { default: "20px", [site.narrow]: "16px" },
    },
    avatar: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: calcom.card,
        display: "inline-flex",
        fontSize: "12px",
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    host: {
        color: calcom.muted,
        fontSize: "13px",
    },
    title: {
        fontFamily: calcom.face,
        fontSize: "20px",
        fontWeight: 600,
        marginBlockEnd: "4px",
    },
    detail: {
        alignItems: "center",
        color: calcom.ink,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
    },
    month: {
        alignContent: "start",
        display: "grid",
        gap: "14px",
        padding: "20px",
    },
    monthHead: {
        alignItems: "center",
        display: "flex",
        fontFamily: calcom.face,
        fontSize: "15px",
        justifyContent: "space-between",
    },
    year: {
        color: calcom.muted,
        fontWeight: 500,
    },
    steps: {
        color: calcom.muted,
        display: "flex",
        gap: "12px",
    },
    grid: {
        display: "grid",
        gap: "4px",
        gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
    },
    weekday: {
        color: calcom.muted,
        fontSize: "11px",
        fontWeight: 500,
        paddingBlockEnd: "4px",
        textAlign: "center",
    },
    date: {
        alignItems: "center",
        borderRadius: "6px",
        height: "40px",
        color: calcom.muted,
        display: "flex",
        fontWeight: 500,
        justifyContent: "center",
        position: "relative",
    },
    dateOpen: {
        backgroundColor: calcom.open,
        color: calcom.ink,
        fontWeight: 600,
    },
    dateChosen: {
        backgroundColor: calcom.chosen,
        color: calcom.onChosen,
    },
    today: {
        backgroundColor: "currentColor",
        borderRadius: "50%",
        bottom: "5px",
        height: "4px",
        position: "absolute",
        width: "4px",
    },
    times: {
        alignContent: "start",
        borderInlineStartColor: calcom.line,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: { default: "grid", [site.narrow]: "none" },
        gap: "8px",
        padding: "20px",
    },
    timesHead: {
        alignItems: "center",
        whiteSpace: "nowrap",
        display: "flex",
        fontFamily: calcom.face,
        fontSize: "15px",
        justifyContent: "space-between",
        paddingBlockEnd: "4px",
    },
    toggle: {
        borderColor: calcom.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: calcom.muted,
        display: "flex",
        fontFamily: calcom.text,
        fontSize: "11px",
        gap: "2px",
        padding: "2px",
    },
    toggleOn: {
        backgroundColor: calcom.open,
        borderRadius: "4px",
        color: calcom.ink,
        paddingInline: "4px",
    },
    time: {
        alignItems: "center",
        borderColor: calcom.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        fontWeight: 600,
        gap: "8px",
        justifyContent: "center",
        paddingBlock: "8px",
    },
    dot: {
        backgroundColor: calcom.available,
        borderRadius: "50%",
        height: "6px",
        width: "6px",
    },
    brand: {
        color: calcom.muted,
        fontFamily: calcom.face,
        fontSize: "13px",
        fontWeight: 600,
    },
});
