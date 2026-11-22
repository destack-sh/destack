import * as style from "@destack/style";

import { useStory } from "../verb/story";
import { monday } from "./monday.stylex";

/** The free slots the page finds in your week. */
const slots: readonly string[] = ["Tue 9–11", "Wed 13–14", "Thu 10–12"];

/** The page's figures: a number and what it counts. */
const figures: readonly (readonly [value: string, label: string])[] = [
    ["3", "promises"],
    ["4", "due"],
    ["6h", "free"],
];

/** Draw the Monday page an agent made on its own: its stock look, on sample data. */
export function MadeMonday() {
    // read the story the page tells
    const story = useStory();

    return (
        <div data-component="MadeMondayPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.header)}>
                <span {...style.attrs(styles.stack)}>
                    <b {...style.attrs(styles.title)}>📅 My Monday</b>
                    <span {...style.attrs(styles.muted)}>Week of {story().label(0)}</span>
                </span>
                <span data-xray="Identity" {...style.attrs(styles.button)}>
                    Sign in with Google
                </span>
            </header>

            {/* figures */}
            <div {...style.attrs(styles.figures)}>
                {figures.map(([value, label]) => (
                    <span {...style.attrs(styles.card, styles.figure)}>
                        <b {...style.attrs(styles.value)}>{value}</b>
                        <span {...style.attrs(styles.muted)}>{label}</span>
                    </span>
                ))}
            </div>

            {/* promises */}
            <section data-xray="Database" {...style.attrs(styles.card)}>
                <b {...style.attrs(styles.heading)}>✉️ Promises from email</b>
                {story()
                    .commitments.filter(([, , , , state]) => state !== "kept")
                    .slice(0, 3)
                    .map(([person, what, , , state]) => (
                        <span {...style.attrs(styles.row)}>
                            <span {...style.attrs(styles.box)} />
                            <span {...style.attrs(styles.stack)}>
                                {what}
                                <span {...style.attrs(styles.muted, styles.small)}>
                                    to {person}
                                </span>
                            </span>
                            <span
                                {...style.attrs(
                                    styles.pill,
                                    state === "open" ? styles.open : styles.waiting,
                                )}
                            >
                                {state === "open" ? "Open" : "Waiting"}
                            </span>
                        </span>
                    ))}
            </section>

            {/* free slots */}
            <section data-xray="Jobs" {...style.attrs(styles.card)}>
                <b {...style.attrs(styles.heading)}>🗓️ Free this week</b>
                <span {...style.attrs(styles.slots)}>
                    {slots.map((slot) => (
                        <span {...style.attrs(styles.slot)}>{slot}</span>
                    ))}
                </span>
            </section>

            {/* where the data comes from */}
            <p {...style.attrs(styles.muted, styles.small, styles.note)}>
                Using sample data · connect Gmail to see your inbox
            </p>
        </div>
    );
}

/** The made Monday page's styles. */
const styles = style.create({
    page: {
        alignContent: "start",
        backgroundColor: monday.surface,
        color: monday.ink,
        display: "grid",
        fontFamily: monday.face,
        fontSize: "13px",
        gap: "10px",
        gridAutoRows: "max-content",
        minHeight: "100%",
        paddingBlock: "16px",
        paddingInline: "16px",
    },
    header: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        justifyContent: "space-between",
    },
    stack: {
        display: "grid",
        gap: "2px",
        minWidth: 0,
    },
    title: {
        fontSize: "20px",
        fontWeight: 700,
        letterSpacing: "-0.01em",
    },
    button: {
        backgroundColor: monday.button,
        borderRadius: "8px",
        color: monday.onAccent,
        fontSize: "12px",
        fontWeight: 600,
        paddingBlock: "6px",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    figures: {
        display: "grid",
        gap: "12px",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
    },
    card: {
        backgroundColor: monday.card,
        borderColor: monday.line,
        borderRadius: "12px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxShadow: monday.shadow,
        display: "grid",
        gap: "6px",
        paddingBlock: "10px",
        paddingInline: "14px",
    },
    figure: {
        alignItems: "baseline",
        display: "flex",
    },
    value: {
        fontSize: "20px",
        fontWeight: 700,
    },
    heading: {
        fontSize: "14px",
        fontWeight: 700,
    },
    row: {
        alignItems: "center",
        display: "grid",
        gap: "10px",
        gridTemplateColumns: "auto minmax(0, 1fr) auto",
    },
    box: {
        borderColor: monday.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "2px",
        height: "16px",
        width: "16px",
    },
    pill: {
        borderRadius: "999px",
        fontSize: "11px",
        fontWeight: 600,
        paddingBlock: "2px",
        paddingInline: "8px",
    },
    open: {
        backgroundColor: monday.openPill,
        color: monday.openInk,
    },
    waiting: {
        backgroundColor: monday.waitingPill,
        color: monday.waitingInk,
    },
    slots: {
        display: "flex",
        flexWrap: "wrap",
        gap: "8px",
    },
    slot: {
        backgroundColor: monday.slot,
        borderRadius: "999px",
        color: monday.slotInk,
        fontWeight: 500,
        paddingBlock: "4px",
        paddingInline: "10px",
    },
    muted: {
        color: monday.muted,
    },
    small: {
        fontSize: "12px",
    },
    note: {
        margin: 0,
    },
});
