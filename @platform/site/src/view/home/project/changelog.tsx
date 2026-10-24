import * as style from "@destack/style";

import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { changelog } from "./changelog.stylex";

/** The launch's releases, newest first: version, date, title and what changed. */
export const releases: readonly (readonly [
    version: string,
    date: string,
    title: string,
    notes: readonly string[],
])[] = [
    [
        "0.9",
        "Oct 20",
        "Pricing",
        ["Plans for people and organisations", "Usage billed by the hour"],
    ],
    ["0.8", "Oct 14", "Waitlist", ["Invites go out in batches of 400"]],
    ["0.7", "Oct 7", "Desktop app", ["Signed builds for macOS", "Updates in the background"]],
];

/** Draw your changelog as a small site you deployed to its own Vercel address. */
export function VercelChangelog() {
    return (
        <div data-component="VercelChangelogPage" {...style.attrs(styles.page)}>
            {/* header */}
            <header {...style.attrs(styles.header)}>
                <b {...style.attrs(styles.title)}>Changelog</b>
                <span {...style.attrs(styles.button)}>Subscribe</span>
            </header>

            {/* releases */}
            <ol {...style.attrs(styles.list)}>
                {releases.map(([version, date, title, notes]) => (
                    <li {...style.attrs(styles.release)}>
                        <span {...style.attrs(styles.date)}>{date}</span>
                        <span {...style.attrs(styles.body)}>
                            <span {...style.attrs(styles.heading)}>
                                <span {...style.attrs(styles.pill)}>v{version}</span>
                                <b>{title}</b>
                            </span>
                            <ul {...style.attrs(styles.notes)}>
                                {notes.map((note) => (
                                    <li>{note}</li>
                                ))}
                            </ul>
                        </span>
                    </li>
                ))}
            </ol>

            {/* the host's corner badge */}
            <span {...style.attrs(styles.badge)}>
                <Favicon icon="vercel" tint={changelog.badgeInk} size={11} />
                Deployed on Vercel
            </span>
        </div>
    );
}

/** The changelog page styles. */
const styles = style.create({
    page: {
        backgroundColor: changelog.paper,
        color: changelog.ink,
        display: "grid",
        alignContent: "start",
        fontFamily: site.system,
        gap: "28px",
        height: "100%",
        overflow: "hidden",
        padding: { default: "40px 56px", [site.narrow]: "24px 20px" },
        position: "relative",
    },
    header: {
        alignItems: "center",
        display: "flex",
        justifyContent: "space-between",
        maxWidth: "560px",
    },
    title: {
        fontSize: "28px",
        fontWeight: 600,
        letterSpacing: "-0.02em",
    },
    button: {
        backgroundColor: changelog.button,
        borderRadius: "6px",
        color: changelog.buttonInk,
        fontSize: "13px",
        fontWeight: 500,
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    list: {
        display: "grid",
        listStyle: "none",
        margin: 0,
        maxWidth: "560px",
        padding: 0,
    },
    release: {
        borderTopColor: changelog.rule,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        display: "grid",
        gap: { default: "24px", [site.narrow]: "8px" },
        gridTemplateColumns: { default: "72px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        paddingBlock: "16px",
    },
    date: {
        color: changelog.muted,
        fontSize: "13px",
        paddingBlockStart: "2px",
    },
    body: {
        display: "grid",
        gap: "6px",
    },
    heading: {
        alignItems: "center",
        display: "flex",
        fontSize: "15px",
        gap: "8px",
    },
    pill: {
        backgroundColor: changelog.pill,
        borderRadius: "999px",
        fontFamily: "ui-monospace, Menlo, monospace",
        fontSize: "11px",
        paddingBlock: "1px",
        paddingInline: "7px",
    },
    notes: {
        color: changelog.muted,
        display: "grid",
        fontSize: "14px",
        gap: "2px",
        margin: 0,
        paddingInlineStart: "18px",
    },
    badge: {
        alignItems: "center",
        backgroundColor: changelog.badge,
        borderRadius: "6px",
        bottom: "12px",
        color: changelog.badgeInk,
        display: "inline-flex",
        fontSize: "11px",
        gap: "6px",
        paddingBlock: "4px",
        paddingInline: "8px",
        position: "absolute",
        right: "12px",
    },
});
