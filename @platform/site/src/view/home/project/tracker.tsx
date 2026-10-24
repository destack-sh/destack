import * as style from "@destack/style";

import { site } from "../figure/site.stylex";
import { tracker } from "./tracker.stylex";

/** The launch's milestones, in order: what, when, and whether it is done. */
export const milestones: readonly (readonly [title: string, due: string, isDone: boolean])[] = [
    ["Pricing page", "Mon 20", true],
    ["Record the demo", "Wed 22", false],
    ["Email the waitlist", "Thu 23", false],
    ["Go live", "Fri 24", false],
];

/** Draw your launch tracker as it runs on your laptop: a dev server, saved in this browser only. */
export function LocalTracker() {
    return (
        <div data-component="LocalTrackerPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.header)}>
                <b {...style.attrs(styles.title)}>Launch tracker</b>
                <span {...style.attrs(styles.countdown)}>3 days to launch</span>
            </header>

            {/* milestones */}
            <ul {...style.attrs(styles.list)}>
                {milestones.map(([title, due, isDone]) => (
                    <li {...style.attrs(styles.item)}>
                        <span {...style.attrs(styles.box, isDone && styles.boxDone)}>
                            {isDone ? "✓" : ""}
                        </span>
                        <span {...style.attrs(styles.name, isDone && styles.nameDone)}>
                            {title}
                        </span>
                        <span {...style.attrs(styles.due)}>{due}</span>
                    </li>
                ))}
            </ul>

            {/* where the data lives */}
            <p {...style.attrs(styles.note)}>Saved in this browser · nothing synced</p>

            {/* the dev server's corner badge */}
            <span {...style.attrs(styles.badge)}>vite · localhost:5173</span>
        </div>
    );
}

/** The local tracker page styles. */
const styles = style.create({
    page: {
        backgroundColor: tracker.paper,
        color: tracker.ink,
        display: "grid",
        alignContent: "start",
        fontFamily: site.system,
        gap: "20px",
        height: "100%",
        overflow: "hidden",
        padding: { default: "40px 48px", [site.narrow]: "24px 20px" },
        position: "relative",
    },
    header: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        gap: "12px",
    },
    title: {
        fontSize: "26px",
        fontWeight: 700,
    },
    countdown: {
        backgroundColor: tracker.chip,
        borderRadius: "4px",
        color: tracker.chipInk,
        fontSize: "13px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    list: {
        borderColor: tracker.rule,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        listStyle: "none",
        margin: 0,
        maxWidth: "440px",
        padding: 0,
    },
    item: {
        alignItems: "center",
        borderBottomColor: tracker.rule,
        borderBottomStyle: { default: "solid", ":last-child": "none" },
        borderBottomWidth: "1px",
        display: "flex",
        fontSize: "15px",
        gap: "12px",
        paddingBlock: "10px",
        paddingInline: "14px",
    },
    box: {
        alignItems: "center",
        borderColor: tracker.box,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "11px",
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    boxDone: {
        backgroundColor: tracker.done,
        borderColor: tracker.done,
        color: tracker.badgeInk,
    },
    name: {
        flexGrow: 1,
    },
    nameDone: {
        color: tracker.box,
        textDecorationLine: "line-through",
    },
    due: {
        color: tracker.muted,
        fontSize: "13px",
    },
    note: {
        color: tracker.muted,
        fontSize: "13px",
        margin: 0,
    },
    badge: {
        backgroundColor: tracker.badge,
        borderRadius: "4px",
        bottom: "12px",
        color: tracker.badgeInk,
        fontFamily: "ui-monospace, Menlo, monospace",
        fontSize: "11px",
        paddingBlock: "3px",
        paddingInline: "7px",
        position: "absolute",
        right: "12px",
    },
});
