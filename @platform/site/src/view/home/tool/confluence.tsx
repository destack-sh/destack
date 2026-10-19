import * as style from "@destack/style";

import { launch } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { confluence } from "./confluence.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    switcher: [
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
    sidebar: ["M3 4h18v16H3z", "M9 4v16"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    plus: ["M12 5v14", "M5 12h14"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
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
    home: ["M4 11 12 4l8 7v9h-5v-6H9v6H4z"],
    clock: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5l3 2"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    page: ["M6 3h9l4 4v14H6z", "M14 3v5h5"],
    open: ["m7 10 5 5 5-5"],
    closed: ["m10 7 5 5-5 5"],
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    share: ["M12 3v12", "m7 8 5-5 5 5", "M5 14v6h14v-6"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    eye: [
        "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z",
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    ],
    info: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 11v6", "M12 7.5h.01"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    smile: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M8.5 14.5a4.5 4.5 0 0 0 7 0",
        "M9 10h.01",
        "M15 10h.01",
    ],
} satisfies Record<string, readonly string[]>;

/** The pages of the launch space, as its tree lists them: title, depth, and its state. */
const tree: readonly (readonly [
    title: string,
    depth: number,
    state: "open" | "closed" | "leaf",
])[] = [
    ["Overview", 0, "leaf"],
    ["Launch", 0, "open"],
    ["Destack launch", 1, "leaf"],
    ["Pricing", 1, "leaf"],
    ["Press kit", 1, "leaf"],
    ["Retros", 0, "closed"],
    ["How we work", 0, "closed"],
];

/** The due date of each launch item, in its order. */
const dues: readonly string[] = ["Oct 20, 2025", "Oct 22, 2025", "Oct 23, 2025", "Oct 24, 2025"];

/** Draw the Destack launch plan as a page of the team wiki, shared with guests by space. */
export function Confluence() {
    return (
        <div data-component="ConfluencePage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.top)}>
                <Mark name="sidebar" size={18} />
                <Mark name="switcher" size={18} />
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="confluence" tint="#1868db" size={22} />
                    Confluence
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={15} />
                    Search
                </span>
                <span {...style.attrs(styles.create)}>
                    <Mark name="plus" size={15} />
                    Create
                </span>
                <span {...style.attrs(styles.topEnd)}>
                    <Mark name="bell" size={18} />
                    <Mark name="help" size={18} />
                    <Mark name="gear" size={18} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* sidebar */}
                <nav {...style.attrs(styles.side)}>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="home" size={16} />
                        For you
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="clock" size={16} />
                        Recent
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="star" size={16} />
                        Starred
                    </span>
                    <span {...style.attrs(styles.space)}>
                        <span {...style.attrs(styles.spaceMark)}>D</span>
                        <b>Destack</b>
                    </span>
                    <span {...style.attrs(styles.group)}>Content</span>
                    {tree.map(([title, depth, state]) => (
                        <span
                            style={{ "padding-inline-start": `${4 + depth * 16}px` }}
                            {...style.attrs(
                                styles.node,
                                title === "Destack launch" && styles.nodeOn,
                            )}
                        >
                            <span {...style.attrs(styles.twist)}>
                                {state === "leaf" ? undefined : (
                                    <Mark name={state === "open" ? "open" : "closed"} size={12} />
                                )}
                            </span>
                            <Mark name="page" size={15} />
                            {title}
                        </span>
                    ))}
                </nav>

                {/* the page */}
                <main {...style.attrs(styles.main)}>
                    <span {...style.attrs(styles.bar)}>
                        <span {...style.attrs(styles.crumbs)}>Spaces / Destack / Launch</span>
                        <span {...style.attrs(styles.actions)}>
                            <Mark name="pencil" size={16} />
                            <span {...style.attrs(styles.shareButton)}>
                                <Mark name="share" size={14} />
                                Share
                            </span>
                            <Mark name="more" size={16} />
                        </span>
                    </span>
                    <article {...style.attrs(styles.article)}>
                        <span aria-hidden="true" {...style.attrs(styles.emoji)}>
                            🚀
                        </span>
                        <b data-morph-title {...style.attrs(styles.title)}>
                            Destack launch
                        </b>
                        <span {...style.attrs(styles.byline)}>
                            <span {...style.attrs(styles.avatar)}>F</span>
                            Owned by Florian
                            <span {...style.attrs(styles.dot)}>·</span>
                            Last updated 2 hours ago
                            <span {...style.attrs(styles.dot)}>·</span>
                            <Mark name="eye" size={14} />
                            14 people viewed
                        </span>

                        <table {...style.attrs(styles.table)}>
                            <tbody>
                                <tr>
                                    <th {...style.attrs(styles.cellHead)}>Status</th>
                                    <td {...style.attrs(styles.cell)}>
                                        <span {...style.attrs(styles.lozenge)}>In progress</span>
                                    </td>
                                </tr>
                                <tr>
                                    <th {...style.attrs(styles.cellHead)}>Launch</th>
                                    <td {...style.attrs(styles.cell)}>
                                        <span {...style.attrs(styles.date)}>
                                            <Mark name="calendar" size={12} />
                                            Oct 24, 2025
                                        </span>
                                    </td>
                                </tr>
                                <tr>
                                    <th {...style.attrs(styles.cellHead)}>Demo</th>
                                    <td {...style.attrs(styles.cell)}>
                                        <span {...style.attrs(styles.date)}>
                                            <Mark name="calendar" size={12} />
                                            Oct 22, 2025 11:00
                                        </span>{" "}
                                        <span {...style.attrs(styles.mention)}>@Ines Ferreira</span>
                                    </td>
                                </tr>
                                <tr>
                                    <th {...style.attrs(styles.cellHead)}>Owner</th>
                                    <td {...style.attrs(styles.cell)}>
                                        <span {...style.attrs(styles.mention)}>@Florian</span>
                                    </td>
                                </tr>
                            </tbody>
                        </table>

                        <p {...style.attrs(styles.panel)}>
                            <span {...style.attrs(styles.panelMark)}>
                                <Mark name="info" size={16} />
                            </span>
                            Ship to the waitlist on Thursday, then open signups on Friday.
                        </p>

                        <b {...style.attrs(styles.heading)}>Before launch</b>
                        <ul {...style.attrs(styles.list)}>
                            {launch.map(([item, isDone], index) => (
                                <li {...style.attrs(styles.item)}>
                                    <span {...style.attrs(styles.box, isDone && styles.boxDone)}>
                                        {isDone ? (
                                            <svg viewBox="0 0 24 24" {...style.attrs(styles.tick)}>
                                                <path d="m6 12.5 4 4 8-9" />
                                            </svg>
                                        ) : undefined}
                                    </span>
                                    <span {...style.attrs(isDone && styles.done)}>{item}</span>
                                    <span {...style.attrs(styles.mention)}>@Florian</span>
                                    <span {...style.attrs(styles.date, styles.wide)}>
                                        {dues[index]}
                                    </span>
                                </li>
                            ))}
                        </ul>

                        <span {...style.attrs(styles.reactions)}>
                            <Mark name="smile" size={16} />
                            <span {...style.attrs(styles.reaction)}>👍 3</span>
                            <span {...style.attrs(styles.reaction)}>🚀 2</span>
                        </span>
                    </article>
                </main>
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

/** The Confluence page styles. */
const styles = style.create({
    page: {
        backgroundColor: confluence.surface,
        color: confluence.ink,
        display: "grid",
        fontFamily: confluence.face,
        fontSize: "14px",
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
        strokeWidth: 1.7,
    },

    // top bar
    top: {
        alignItems: "center",
        borderBottomColor: confluence.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: confluence.muted,
        display: "flex",
        gap: "12px",
        height: "48px",
        paddingInline: "12px",
    },
    brand: {
        alignItems: "center",
        color: confluence.ink,
        display: "flex",
        fontSize: "16px",
        fontWeight: 600,
        gap: "6px",
    },
    search: {
        alignItems: "center",
        borderColor: confluence.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: { default: "flex", [site.narrow]: "none" },
        flexGrow: 1,
        gap: "8px",
        height: "32px",
        marginInlineStart: "24px",
        maxWidth: "420px",
        paddingInline: "10px",
    },
    create: {
        alignItems: "center",
        backgroundColor: confluence.blue,
        borderRadius: "4px",
        color: confluence.onAccent,
        display: "inline-flex",
        fontWeight: 500,
        gap: "4px",
        height: "32px",
        paddingInline: "10px",
    },
    topEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: confluence.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "232px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // sidebar
    side: {
        backgroundColor: confluence.side,
        borderInlineEndColor: confluence.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: confluence.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "2px",
        paddingBlock: "12px",
        paddingInline: "8px",
    },
    link: {
        alignItems: "center",
        borderRadius: "4px",
        display: "flex",
        gap: "10px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    space: {
        alignItems: "center",
        color: confluence.ink,
        display: "flex",
        gap: "8px",
        marginBlockStart: "12px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    spaceMark: {
        alignItems: "center",
        backgroundColor: site.space,
        borderRadius: "4px",
        color: confluence.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        fontWeight: 700,
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    group: {
        color: confluence.faint,
        fontSize: "12px",
        fontWeight: 600,
        paddingBlockEnd: "4px",
        paddingBlockStart: "8px",
        paddingInline: "8px",
    },
    node: {
        alignItems: "center",
        borderRadius: "4px",
        display: "flex",
        gap: "6px",
        paddingBlock: "5px",
        paddingInlineEnd: "8px",
        whiteSpace: "nowrap",
    },
    nodeOn: {
        backgroundColor: confluence.selected,
        color: confluence.blue,
        fontWeight: 500,
    },
    twist: {
        display: "inline-flex",
        justifyContent: "center",
        width: "14px",
    },

    // the page
    main: {
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minWidth: 0,
    },
    bar: {
        alignItems: "center",
        color: confluence.faint,
        display: "flex",
        paddingBlock: "10px",
        paddingInline: { default: "40px", [site.narrow]: "16px" },
    },
    crumbs: {
        fontSize: "13px",
    },
    actions: {
        alignItems: "center",
        color: confluence.muted,
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    shareButton: {
        alignItems: "center",
        backgroundColor: confluence.blue,
        borderRadius: "4px",
        color: confluence.onAccent,
        display: "inline-flex",
        fontWeight: 500,
        gap: "6px",
        height: "30px",
        paddingInline: "10px",
    },
    article: {
        alignContent: "start",
        display: "grid",
        gap: "14px",
        overflow: "hidden",
        paddingBlockEnd: "20px",
        paddingInline: { default: "40px", [site.narrow]: "16px" },
    },
    emoji: {
        fontSize: "40px",
        lineHeight: 1,
        marginBlockStart: "8px",
    },
    title: {
        fontSize: "29px",
        fontWeight: 650,
        letterSpacing: "-0.01em",
    },
    byline: {
        alignItems: "center",
        color: confluence.faint,
        display: "flex",
        flexWrap: "wrap",
        fontSize: "13px",
        gap: "6px",
    },
    avatar: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: confluence.onAccent,
        display: "inline-flex",
        fontSize: "9px",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    dot: {
        color: confluence.line,
    },
    table: {
        borderCollapse: "collapse",
        maxWidth: "480px",
        width: "100%",
    },
    cellHead: {
        backgroundColor: confluence.header,
        borderColor: confluence.line,
        borderStyle: "solid",
        borderWidth: "1px",
        fontWeight: 600,
        paddingBlock: "8px",
        paddingInline: "10px",
        textAlign: "start",
        width: "120px",
    },
    cell: {
        borderColor: confluence.line,
        borderStyle: "solid",
        borderWidth: "1px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    lozenge: {
        backgroundColor: confluence.lozenge,
        borderRadius: "3px",
        color: confluence.lozengeInk,
        fontSize: "11px",
        fontWeight: 700,
        paddingBlock: "1px",
        paddingInline: "4px",
        textTransform: "uppercase",
    },
    date: {
        alignItems: "center",
        backgroundColor: confluence.date,
        borderRadius: "3px",
        display: "inline-flex",
        fontSize: "12px",
        gap: "4px",
        paddingBlock: "1px",
        paddingInline: "6px",
        whiteSpace: "nowrap",
    },
    mention: {
        backgroundColor: confluence.selected,
        borderRadius: "999px",
        color: confluence.blue,
        fontSize: "13px",
        paddingBlock: "1px",
        paddingInline: "8px",
        whiteSpace: "nowrap",
    },
    panel: {
        alignItems: "start",
        backgroundColor: confluence.panel,
        borderRadius: "4px",
        display: "flex",
        gap: "10px",
        lineHeight: 1.5,
        margin: 0,
        paddingBlock: "10px",
        paddingInline: "14px",
    },
    panelMark: {
        color: confluence.panelMark,
        display: "flex",
        paddingBlockStart: "2px",
    },
    heading: {
        fontSize: "20px",
        fontWeight: 650,
        marginBlockStart: "4px",
    },
    list: {
        display: "grid",
        gap: "8px",
        listStyle: "none",
        margin: 0,
        padding: 0,
    },
    item: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    box: {
        alignItems: "center",
        borderColor: confluence.muted,
        borderRadius: "3px",
        borderStyle: "solid",
        borderWidth: "1.5px",
        boxSizing: "border-box",
        display: "inline-flex",
        flexShrink: 0,
        height: "15px",
        justifyContent: "center",
        width: "15px",
    },
    boxDone: {
        backgroundColor: confluence.blue,
        borderColor: confluence.blue,
    },
    tick: {
        fill: "none",
        height: "11px",
        stroke: confluence.onAccent,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 3.5,
        width: "11px",
    },
    done: {
        color: confluence.faint,
        textDecorationLine: "line-through",
    },
    wide: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },
    reactions: {
        alignItems: "center",
        color: confluence.faint,
        display: "flex",
        gap: "8px",
        marginBlockStart: "6px",
    },
    reaction: {
        borderColor: confluence.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: confluence.ink,
        fontSize: "13px",
        paddingBlock: "2px",
        paddingInline: "8px",
    },
});
