import * as style from "@destack/style";

import { launch } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { obsidian } from "./obsidian.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    sidebar: ["M3 4h18v16H3z", "M9 4v16"],
    files: ["M4 6a1 1 0 0 1 1-1h5l2 2h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    bookmark: ["M6 3h12v18l-6-4-6 4z"],
    graph: [
        "M6 6h.01",
        "M18 8h.01",
        "M8 18h.01",
        "M17 17h.01",
        "M6 6l12 2",
        "M6 6l2 12",
        "M18 8l-1 9",
        "M8 18l9-1",
    ],
    canvas: ["M4 4h7v7H4z", "M13 13h7v7h-7z", "M11 7.5h4a2 2 0 0 1 2 2V13"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
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
    note: ["M6 3h9l4 4v14H6z", "M14 3v5h5"],
    folder: ["M4 6a1 1 0 0 1 1-1h5l2 2h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"],
    sort: ["M7 4v16", "m3 8 4-4 4 4", "M17 20V4", "m13 16 4 4 4-4"],
    collapse: ["m7 15 5-5 5 5"],
    caret: ["m9 6 6 6-6 6"],
    open: ["m7 10 5 5 5-5"],
    close: ["M18 6 6 18", "m6 6 12 12"],
    plus: ["M12 5v14", "M5 12h14"],
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
    book: ["M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z", "M4 19V5"],
    more: ["M12 5h.01", "M12 12h.01", "M12 19h.01"],
    text: ["M4 6h16", "M4 12h16", "M4 18h10"],
    list: ["M8 6h12", "M8 12h12", "M8 18h12", "M4 6h.01", "M4 12h.01", "M4 18h.01"],
    flame: ["M12 3c1 4 5 6 5 11a5 5 0 0 1-10 0c0-3 2-4 2-7 1 1 2 2 3 2 0-2-1-4 0-6z"],
    sync: ["M20 12a8 8 0 1 1-2.3-5.7L20 8", "M20 4v4h-4"],
} satisfies Record<string, readonly string[]>;

/** The vault's files, as the explorer lists them: name, depth, and whether it is a folder. */
const files: readonly (readonly [name: string, depth: number, isFolder: boolean])[] = [
    ["Launch", 0, true],
    ["Destack launch", 1, false],
    ["Pricing", 1, false],
    ["Press kit", 1, false],
    ["Meetings", 0, true],
    ["Retros", 0, true],
    ["Templates", 0, true],
    ["Inbox", 0, false],
];

/** The note's properties: name, type glyph and value. */
const fields: readonly (readonly [name: string, glyph: keyof typeof glyphs, value: string])[] = [
    ["status", "text", "in progress"],
    ["launch", "calendar", "2025-10-24"],
    ["demo", "calendar", "2025-10-22 11:00"],
    ["owner", "text", "Florian"],
];

/** Draw the Destack launch plan as a note in a synced vault, in the app's default dark theme. */
export function Obsidian() {
    return (
        <div data-component="ObsidianPage" {...style.attrs(styles.page)}>
            {/* ribbon */}
            <nav {...style.attrs(styles.ribbon)}>
                <Mark name="sidebar" size={18} />
                <Mark name="files" size={18} />
                <Mark name="search" size={18} />
                <Mark name="bookmark" size={18} />
                <Mark name="graph" size={18} />
                <Mark name="canvas" size={18} />
                <Mark name="calendar" size={18} />
                <span {...style.attrs(styles.ribbonEnd)}>
                    <Mark name="help" size={18} />
                    <Mark name="gear" size={18} />
                </span>
            </nav>

            {/* file explorer */}
            <nav {...style.attrs(styles.explorer)}>
                <span {...style.attrs(styles.explorerTools)}>
                    <Mark name="note" size={16} />
                    <Mark name="folder" size={16} />
                    <Mark name="sort" size={16} />
                    <Mark name="collapse" size={16} />
                </span>
                {files.map(([name, depth, isFolder]) => (
                    <span
                        style={{ "padding-inline-start": `${12 + depth * 18}px` }}
                        {...style.attrs(
                            styles.file,
                            isFolder && styles.folder,
                            name === "Destack launch" && styles.fileOn,
                        )}
                    >
                        {isFolder ? (
                            <Mark name={name === "Launch" ? "open" : "caret"} size={12} />
                        ) : undefined}
                        {name}
                    </span>
                ))}
                <span {...style.attrs(styles.vault)}>
                    <Mark name="open" size={12} />
                    Destack
                    <span {...style.attrs(styles.vaultEnd)}>
                        <Mark name="help" size={14} />
                        <Mark name="gear" size={14} />
                    </span>
                </span>
            </nav>

            {/* the note in its tab */}
            <div {...style.attrs(styles.pane)}>
                <span {...style.attrs(styles.tabs)}>
                    <span {...style.attrs(styles.tab)}>
                        <span {...style.attrs(styles.tabName)}>Destack launch</span>
                        <Mark name="close" size={12} />
                    </span>
                    <span {...style.attrs(styles.tabQuiet)}>
                        <span {...style.attrs(styles.tabName)}>Pricing</span>
                    </span>
                    <span {...style.attrs(styles.newTab)}>
                        <Mark name="plus" size={14} />
                    </span>
                </span>
                <span {...style.attrs(styles.viewHead)}>
                    <Mark name="back" size={16} />
                    <Mark name="forward" size={16} />
                    <span {...style.attrs(styles.crumbs)}>
                        Launch
                        <span {...style.attrs(styles.crumbRule)}>/</span>
                        <span {...style.attrs(styles.crumbOn)}>Destack launch</span>
                    </span>
                    <Mark name="book" size={16} />
                    <Mark name="more" size={16} />
                </span>

                <article {...style.attrs(styles.note)}>
                    <b data-morph-title {...style.attrs(styles.title)}>
                        Destack launch
                    </b>
                    <span {...style.attrs(styles.propertiesHead)}>Properties</span>
                    <dl {...style.attrs(styles.properties)}>
                        {fields.map(([name, glyph, value]) => (
                            <>
                                <dt {...style.attrs(styles.key)}>
                                    <Mark name={glyph} size={14} />
                                    {name}
                                </dt>
                                <dd {...style.attrs(styles.value)}>{value}</dd>
                            </>
                        ))}
                        <dt {...style.attrs(styles.key)}>
                            <Mark name="list" size={14} />
                            tags
                        </dt>
                        <dd {...style.attrs(styles.value)}>
                            <span {...style.attrs(styles.tag)}>launch</span>
                            <span {...style.attrs(styles.tag)}>destack</span>
                        </dd>
                    </dl>
                    <p {...style.attrs(styles.callout)}>
                        <b {...style.attrs(styles.calloutTitle)}>
                            <Mark name="flame" size={16} />
                            Tip
                        </b>
                        Ship to the waitlist on Thursday, then open signups on Friday.
                    </p>
                    <b {...style.attrs(styles.heading)}>Before launch</b>
                    <ul {...style.attrs(styles.list)}>
                        {launch.map(([item, isDone]) => (
                            <li {...style.attrs(styles.item)}>
                                <span {...style.attrs(styles.box, isDone && styles.boxDone)}>
                                    {isDone ? (
                                        <svg viewBox="0 0 24 24" {...style.attrs(styles.tick)}>
                                            <path d="m6 12.5 4 4 8-9" />
                                        </svg>
                                    ) : undefined}
                                </span>
                                <span {...style.attrs(isDone && styles.done)}>{item}</span>
                            </li>
                        ))}
                    </ul>
                    <span {...style.attrs(styles.link)}>[[Pricing]] · [[Press kit]]</span>
                </article>

                <span {...style.attrs(styles.status)}>
                    <span>3 backlinks</span>
                    <span>52 words</span>
                    <span>318 characters</span>
                    <span {...style.attrs(styles.sync)}>
                        <Mark name="sync" size={13} />
                    </span>
                </span>
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

/** The Obsidian page styles. */
const styles = style.create({
    page: {
        backgroundColor: obsidian.editor,
        color: obsidian.ink,
        colorScheme: "dark",
        display: "grid",
        fontFamily: obsidian.face,
        fontSize: "14px",
        gridTemplateColumns: {
            default: "44px 220px minmax(0, 1fr)",
            [site.narrow]: "minmax(0, 1fr)",
        },
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
        strokeWidth: 1.75,
    },

    // ribbon
    ribbon: {
        alignItems: "center",
        backgroundColor: obsidian.side,
        borderInlineEndColor: obsidian.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: obsidian.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "16px",
        paddingBlock: "12px",
    },
    ribbonEnd: {
        display: "flex",
        flexDirection: "column",
        gap: "16px",
        marginBlockStart: "auto",
    },

    // file explorer
    explorer: {
        backgroundColor: obsidian.side,
        borderInlineEndColor: obsidian.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: obsidian.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        fontSize: "13px",
        gap: "1px",
        paddingInline: "6px",
    },
    explorerTools: {
        display: "flex",
        gap: "14px",
        justifyContent: "center",
        paddingBlock: "12px",
    },
    file: {
        alignItems: "center",
        borderRadius: "4px",
        display: "flex",
        gap: "4px",
        height: "26px",
        paddingInlineEnd: "8px",
    },
    folder: {
        color: obsidian.ink,
    },
    fileOn: {
        backgroundColor: obsidian.active,
        color: obsidian.heading,
    },
    vault: {
        alignItems: "center",
        borderTopColor: obsidian.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        color: obsidian.ink,
        display: "flex",
        fontWeight: 600,
        gap: "6px",
        marginBlockStart: "auto",
        marginInline: "-6px",
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    vaultEnd: {
        color: obsidian.muted,
        display: "flex",
        gap: "10px",
        marginInlineStart: "auto",
    },

    // the note
    pane: {
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr) auto",
        minWidth: 0,
    },
    tabs: {
        alignItems: "end",
        backgroundColor: obsidian.side,
        display: "flex",
        gap: "2px",
        height: "40px",
        paddingInline: "8px",
    },
    tab: {
        alignItems: "center",
        backgroundColor: obsidian.tab,
        borderStartEndRadius: "8px",
        borderStartStartRadius: "8px",
        color: obsidian.heading,
        display: "flex",
        fontSize: "13px",
        gap: "8px",
        height: "32px",
        maxWidth: "200px",
        minWidth: 0,
        paddingInline: "12px",
    },
    tabQuiet: {
        alignItems: "center",
        color: obsidian.muted,
        display: "flex",
        fontSize: "13px",
        height: "32px",
        maxWidth: "160px",
        minWidth: 0,
        paddingInline: "12px",
    },
    tabName: {
        flexGrow: 1,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    newTab: {
        alignItems: "center",
        color: obsidian.muted,
        display: "flex",
        height: "32px",
        paddingInline: "6px",
    },
    viewHead: {
        alignItems: "center",
        color: obsidian.faint,
        display: "flex",
        gap: "12px",
        height: "38px",
        paddingInline: "12px",
    },
    crumbs: {
        alignItems: "center",
        color: obsidian.faint,
        display: "flex",
        flexGrow: 1,
        fontSize: "13px",
        gap: "6px",
        justifyContent: "center",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    crumbRule: {
        color: obsidian.line,
    },
    crumbOn: {
        color: obsidian.muted,
    },
    note: {
        alignContent: "start",
        boxSizing: "border-box",
        display: "grid",
        gap: "12px",
        justifySelf: "center",
        maxWidth: "640px",
        overflow: "hidden",
        paddingBlock: "20px",
        paddingInline: "32px",
        width: "100%",
    },
    title: {
        color: obsidian.heading,
        fontSize: "30px",
        fontWeight: 700,
        letterSpacing: "-0.015em",
    },
    propertiesHead: {
        color: obsidian.muted,
        fontSize: "13px",
        fontWeight: 600,
    },
    properties: {
        borderBottomColor: obsidian.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        columnGap: "12px",
        display: "grid",
        fontSize: "13px",
        gridTemplateColumns: "120px minmax(0, 1fr)",
        margin: 0,
        paddingBlockEnd: "14px",
        rowGap: "8px",
    },
    key: {
        alignItems: "center",
        color: obsidian.faint,
        display: "flex",
        gap: "8px",
    },
    value: {
        display: "flex",
        gap: "6px",
        margin: 0,
    },
    tag: {
        backgroundColor: obsidian.tagFill,
        borderRadius: "999px",
        color: obsidian.accent,
        fontSize: "12px",
        paddingBlock: "1px",
        paddingInline: "8px",
    },
    callout: {
        backgroundColor: obsidian.tipFill,
        borderRadius: "4px",
        display: "grid",
        gap: "4px",
        lineHeight: 1.55,
        margin: 0,
        paddingBlock: "12px",
        paddingInline: "16px",
    },
    calloutTitle: {
        alignItems: "center",
        color: obsidian.tip,
        display: "flex",
        gap: "8px",
    },
    heading: {
        color: obsidian.heading,
        fontSize: "22px",
        fontWeight: 650,
        marginBlockStart: "4px",
    },
    list: {
        display: "grid",
        gap: "6px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    item: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        lineHeight: 1.55,
    },
    box: {
        alignItems: "center",
        borderColor: obsidian.faint,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        boxSizing: "border-box",
        display: "inline-flex",
        flexShrink: 0,
        height: "15px",
        justifyContent: "center",
        width: "15px",
    },
    boxDone: {
        backgroundColor: obsidian.accent,
        borderColor: obsidian.accent,
    },
    tick: {
        fill: "none",
        height: "11px",
        stroke: obsidian.onAccent,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 3.5,
        width: "11px",
    },
    done: {
        color: obsidian.faint,
        textDecorationLine: "line-through",
    },
    link: {
        color: obsidian.accent,
        fontSize: "14px",
    },
    status: {
        alignItems: "center",
        backgroundColor: obsidian.side,
        borderStartStartRadius: "6px",
        color: obsidian.faint,
        display: "flex",
        fontSize: "11px",
        gap: "14px",
        justifySelf: "end",
        paddingBlock: "3px",
        paddingInline: "10px",
    },
    sync: {
        display: "flex",
    },
});
