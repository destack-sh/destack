import * as style from "@destack/style";

import { month, times } from "../verb/corner";
import { Glyph } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { palette } from "../../palette.stylex";
import { calendly } from "./calendly.stylex";

/** Draw your booking page as a scheduling service's card, with a day chosen. */
export function Calendly() {
    return (
        <div data-component="CalendlyPage" {...style.attrs(styles.calendly)}>
            <div {...style.attrs(styles.calCard)}>
                <div {...style.attrs(styles.calHost)}>
                    <span {...style.attrs(styles.calAvatar)}>F</span>
                    <b {...style.attrs(styles.calName)}>Florian</b>
                    <b {...style.attrs(styles.calEvent)}>Destack demo</b>
                    <span {...style.attrs(styles.calDetail)}>
                        <Glyph name="clock" size={20} />
                        30 min
                    </span>
                    <span {...style.attrs(styles.calDetail)}>
                        <Glyph name="video" size={20} />
                        Web conferencing details provided upon confirmation.
                    </span>
                </div>
                <div {...style.attrs(styles.calPicker)}>
                    <b {...style.attrs(styles.calTitle)}>Select a Date &amp; Time</b>
                    <span {...style.attrs(styles.calMonth)}>
                        <span {...style.attrs(styles.calStep, styles.calStepOff)}>
                            <Glyph name="back" size={18} />
                        </span>
                        October 2025
                        <span {...style.attrs(styles.calStep)}>
                            <Glyph name="forward" size={18} />
                        </span>
                    </span>
                    <div {...style.attrs(styles.calGrid)}>
                        {["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"].map((day) => (
                            <span {...style.attrs(styles.calWeekday)}>{day}</span>
                        ))}
                        {Array.from({ length: month.first }, () => (
                            <span />
                        ))}
                        {Array.from({ length: month.days }, (_, index) => index + 1).map((date) => (
                            <span
                                {...style.attrs(
                                    styles.calDate,
                                    month.open.includes(date) && styles.calOpen,
                                    date === month.chosen && styles.calChosen,
                                )}
                            >
                                {date}
                                {date === month.today ? (
                                    <span {...style.attrs(styles.calToday)} />
                                ) : undefined}
                            </span>
                        ))}
                    </div>
                    <span {...style.attrs(styles.calZone)}>
                        <b>Time zone</b>
                        <span {...style.attrs(styles.calZoneValue)}>
                            <Glyph name="globe" size={16} />
                            Central European Time (16:40)
                            <span {...style.attrs(styles.calCaret)} />
                        </span>
                    </span>
                </div>
                <div {...style.attrs(styles.calTimes)}>
                    <span {...style.attrs(styles.calDay)}>Tuesday, October 28</span>
                    {times.map((time) => (
                        <span {...style.attrs(styles.calTime)}>{time}</span>
                    ))}
                </div>
                <span data-component="VendorBadge" {...style.attrs(styles.calRibbon)}>
                    <span {...style.attrs(styles.calPowered)}>powered by</span>
                    Calendly
                </span>
            </div>
        </div>
    );
}

/** The calendly page styles. */
const styles = style.create({
    calendly: {
        backgroundColor: calendly.mist,
        color: calendly.ink,
        display: "flex",
        fontFamily: calendly.face,
        justifyContent: "center",
        height: "100%",
        overflow: "hidden",
        paddingBlock: { default: "40px", [site.narrow]: "16px" },
        paddingInline: { default: "32px", [site.narrow]: "12px" },
    },
    calCard: {
        alignSelf: "start",
        backgroundColor: calendly.surface,
        borderColor: `color-mix(in srgb, ${calendly.ink} 10%, transparent)`,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: "0 1px 8px color-mix(in srgb, black 8%, transparent)",
        display: "grid",
        gridTemplateColumns: {
            default: "220px minmax(0, 1fr) 168px",
            [site.narrow]: "minmax(0, 1fr)",
        },
        overflow: "hidden",
        position: "relative",
        width: "100%",
    },
    calHost: {
        alignContent: "start",
        borderRightColor: `color-mix(in srgb, ${calendly.ink} 10%, transparent)`,
        borderBottomColor: `color-mix(in srgb, ${calendly.ink} 10%, transparent)`,
        borderBottomStyle: "solid",
        borderRightStyle: "solid",
        borderRightWidth: { default: "1px", [site.narrow]: 0 },
        display: "grid",
        paddingBlock: "28px",
        paddingInline: "24px",
        borderBottomWidth: { default: 0, [site.narrow]: "1px" },
    },
    calAvatar: {
        alignItems: "center",
        backgroundColor: palette.teal,
        borderRadius: "50%",
        color: "white",
        display: "flex",
        fontSize: "26px",
        fontWeight: 700,
        height: "64px",
        justifyContent: "center",
        marginBottom: "16px",
        width: "64px",
    },
    calName: {
        color: `color-mix(in srgb, ${calendly.ink} 61%, transparent)`,
        fontSize: "16px",
        fontWeight: 700,
    },
    calEvent: {
        color: calendly.navy,
        fontSize: "28px",
        fontWeight: 700,
        lineHeight: 1.25,
        marginBottom: "24px",
        marginTop: "4px",
    },
    calDetail: {
        alignItems: "flex-start",
        color: `color-mix(in srgb, ${calendly.ink} 61%, transparent)`,
        display: "flex",
        fontSize: "15px",
        fontWeight: 700,
        gap: "8px",
        lineHeight: 1.4,
        marginBottom: "16px",
    },
    calPicker: {
        alignContent: "start",
        display: "grid",
        paddingBlock: "28px",
        paddingInlineStart: "24px",
        paddingInlineEnd: "20px",
    },
    calTitle: {
        color: calendly.navy,
        fontSize: "20px",
        fontWeight: 700,
        marginBottom: "24px",
    },
    calMonth: {
        alignItems: "center",
        display: "flex",
        fontSize: "16px",
        gap: "24px",
        justifyContent: "center",
        marginBottom: "20px",
    },
    calStep: {
        alignItems: "center",
        backgroundColor: `color-mix(in srgb, ${calendly.blue} 6.5%, transparent)`,
        borderRadius: "50%",
        color: calendly.deepBlue,
        display: "flex",
        height: "36px",
        justifyContent: "center",
        width: "36px",
    },
    calStepOff: {
        backgroundColor: "transparent",
        color: `color-mix(in srgb, ${calendly.ink} 40%, transparent)`,
    },
    calGrid: {
        display: "grid",
        gridTemplateColumns: "repeat(7, minmax(0, 1fr))",
        justifyItems: "center",
        rowGap: "6px",
    },
    calWeekday: {
        fontSize: "12px",
        paddingBottom: "4px",
        textAlign: "center",
    },
    calDate: {
        alignItems: "center",
        borderRadius: "50%",
        display: "flex",
        fontSize: "15px",
        height: "40px",
        justifyContent: "center",
        position: "relative",
        width: "40px",
    },
    calOpen: {
        backgroundColor: `color-mix(in srgb, ${calendly.blue} 6.5%, transparent)`,
        color: calendly.deepBlue,
        fontWeight: 700,
    },
    calChosen: {
        backgroundColor: calendly.blue,
        color: "white",
    },
    calToday: {
        backgroundColor: "currentColor",
        borderRadius: "50%",
        bottom: "6px",
        height: "4px",
        position: "absolute",
        width: "4px",
    },
    calZone: {
        display: "grid",
        fontSize: "15px",
        gap: "8px",
        marginTop: "28px",
    },
    calZoneValue: {
        alignItems: "center",
        display: "flex",
        fontSize: "14px",
        gap: "6px",
    },
    calCaret: {
        borderLeft: "4px solid transparent",
        borderRight: "4px solid transparent",
        borderTop: "5px solid currentColor",
        marginLeft: "4px",
    },
    calTimes: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
        paddingBlockStart: { default: "92px", [site.narrow]: "0" },
        paddingBlockEnd: "28px",
        paddingInlineStart: { default: "0", [site.narrow]: "24px" },
        paddingInlineEnd: "20px",
    },
    calDay: {
        fontSize: "16px",
        marginBottom: "6px",
    },
    calTime: {
        borderColor: `color-mix(in srgb, ${calendly.blue} 50%, transparent)`,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: calendly.blue,
        fontSize: "16px",
        fontWeight: 700,
        paddingBlock: "14px",
        textAlign: "center",
    },
    calRibbon: {
        backgroundColor: calendly.slate,
        color: "white",
        display: "grid",
        fontSize: "14px",
        fontWeight: 700,
        justifyItems: "center",
        lineHeight: 1.1,
        paddingBlockStart: "5px",
        paddingBlockEnd: "6px",
        position: "absolute",
        right: "-44px",
        rotate: "45deg",
        top: "22px",
        width: "160px",
    },
    calPowered: {
        fontSize: "8px",
        letterSpacing: "0.04em",
        textTransform: "uppercase",
    },
});
