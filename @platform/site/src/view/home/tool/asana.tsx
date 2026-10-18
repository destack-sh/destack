import * as style from "@destack/style";

import { tasks } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { asana } from "./asana.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    plus: ["M12 5v14", "M5 12h14"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    help: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6",
        "M12 17h.01",
    ],
    home: ["M4 11 12 4l8 7v9h-5v-6H9v6H4z"],
    check: ["m6 12.5 4 4 8-9"],
    circleCheck: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m8 12.5 3 3 5-6"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
    chart: ["M5 20V10", "M12 20V4", "M19 20v-7"],
    target: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z"],
    list: ["M8 6h12", "M8 12h12", "M8 18h12", "M4 6h.01", "M4 12h.01", "M4 18h.01"],
    caret: ["m7 10 5 5 5-5"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    sort: ["M7 4v16", "m3 8 4-4 4 4", "M17 20V4", "m13 16 4 4 4-4"],
    group: ["M4 5h16v5H4z", "M4 14h16v5H4z"],
    lock: ["M7 11V7a5 5 0 0 1 10 0v4", "M5 11h14v10H5z"],
} satisfies Record<string, readonly string[]>;

/** The priority of each launch task, in its order: label and its pill. */
const priorities: readonly (readonly [label: string, level: "high" | "medium" | "low"])[] = [
    ["High", "high"],
    ["High", "high"],
    ["Medium", "medium"],
    ["High", "high"],
    ["Low", "low"],
    ["Medium", "medium"],
];

/** The project's tabs, List chosen. */
const tabs: readonly string[] = [
    "Overview",
    "List",
    "Board",
    "Timeline",
    "Dashboard",
    "Messages",
    "Files",
];

/** Draw the launch tasks as the project's list, in a section for launch week. */
export function Asana() {
    return (
        <div data-component="AsanaPage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.top)}>
                <Mark name="menu" size={18} />
                <span {...style.attrs(styles.create)}>
                    <span {...style.attrs(styles.createMark)}>
                        <Mark name="plus" size={12} />
                    </span>
                    Create
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={15} />
                    Search
                </span>
                <span {...style.attrs(styles.topEnd)}>
                    <Mark name="help" size={18} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* sidebar */}
                <nav {...style.attrs(styles.side)}>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="home" size={16} />
                        Home
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="circleCheck" size={16} />
                        My tasks
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="bell" size={16} />
                        Inbox
                    </span>
                    <span {...style.attrs(styles.group)}>Insights</span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="chart" size={16} />
                        Reporting
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="target" size={16} />
                        Goals
                    </span>
                    <span {...style.attrs(styles.group)}>Projects</span>
                    <span {...style.attrs(styles.link, styles.linkOn)}>
                        <span {...style.attrs(styles.swatch)} />
                        Destack launch
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <span {...style.attrs(styles.swatch, styles.swatchBlue)} />
                        Website
                    </span>
                </nav>

                {/* the project */}
                <main {...style.attrs(styles.main)}>
                    <span {...style.attrs(styles.head)}>
                        <span {...style.attrs(styles.tile)}>
                            <Mark name="list" size={16} />
                        </span>
                        <b {...style.attrs(styles.title)}>Destack launch</b>
                        <Mark name="caret" size={14} />
                        <Mark name="star" size={16} />
                        <span {...style.attrs(styles.status)}>
                            <span {...style.attrs(styles.statusDot)} />
                            On track
                        </span>
                        <span {...style.attrs(styles.headEnd)}>
                            <span {...style.attrs(styles.face, styles.faceFlorian)}>F</span>
                            <span {...style.attrs(styles.face, styles.faceAgent)}>A</span>
                            <span {...style.attrs(styles.share)}>
                                <Mark name="lock" size={13} />
                                Share
                            </span>
                        </span>
                    </span>
                    <span {...style.attrs(styles.tabs)}>
                        {tabs.map((tab) => (
                            <span {...style.attrs(styles.tab, tab === "List" && styles.tabOn)}>
                                {tab}
                            </span>
                        ))}
                    </span>
                    <span {...style.attrs(styles.toolbar)}>
                        <span {...style.attrs(styles.add)}>
                            <Mark name="plus" size={13} />
                            Add task
                            <span {...style.attrs(styles.addCaret)}>
                                <Mark name="caret" size={12} />
                            </span>
                        </span>
                        <span {...style.attrs(styles.toolbarEnd)}>
                            <span {...style.attrs(styles.tool)}>
                                <Mark name="filter" size={14} />
                                Filter
                            </span>
                            <span {...style.attrs(styles.tool)}>
                                <Mark name="sort" size={14} />
                                Sort
                            </span>
                            <span {...style.attrs(styles.tool, styles.wide)}>
                                <Mark name="group" size={14} />
                                Group
                            </span>
                        </span>
                    </span>

                    <div {...style.attrs(styles.table)}>
                        <span {...style.attrs(styles.row, styles.columns)}>
                            <span {...style.attrs(styles.cell)}>Task name</span>
                            <span {...style.attrs(styles.cell, styles.mid)}>Assignee</span>
                            <span {...style.attrs(styles.cell)}>Due date</span>
                            <span {...style.attrs(styles.cell, styles.wide)}>Priority</span>
                        </span>
                        <b {...style.attrs(styles.section)}>
                            <Mark name="caret" size={13} />
                            Launch week
                        </b>
                        {tasks.map(([, title, state, owner, tint, due], index) => (
                            <span {...style.attrs(styles.row)}>
                                <span {...style.attrs(styles.cell, styles.name)}>
                                    <span
                                        {...style.attrs(
                                            styles.check,
                                            state === "done" && styles.checkDone,
                                        )}
                                    >
                                        <Mark name="check" size={10} />
                                    </span>
                                    <span {...style.attrs(styles.taskName)}>{title}</span>
                                </span>
                                <span {...style.attrs(styles.cell, styles.mid)}>
                                    <span
                                        style={{ "background-color": tint }}
                                        {...style.attrs(styles.owner)}
                                    >
                                        {owner}
                                    </span>
                                    {owner === "F" ? "Florian" : "Agent"}
                                </span>
                                <span {...style.attrs(styles.cell, styles.due)}>{due}</span>
                                <span {...style.attrs(styles.cell, styles.wide)}>
                                    <span
                                        {...style.attrs(
                                            styles.pill,
                                            priorities[index]?.[1] === "high"
                                                ? styles.high
                                                : priorities[index]?.[1] === "medium"
                                                  ? styles.medium
                                                  : styles.low,
                                        )}
                                    >
                                        {priorities[index]?.[0]}
                                    </span>
                                </span>
                            </span>
                        ))}
                        <span {...style.attrs(styles.addRow)}>Add task…</span>
                    </div>
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

/** The Asana page styles. */
const styles = style.create({
    page: {
        backgroundColor: asana.chrome,
        color: asana.ink,
        display: "grid",
        fontFamily: asana.face,
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
        strokeWidth: 1.8,
    },

    // top bar
    top: {
        alignItems: "center",
        color: asana.chromeInk,
        display: "flex",
        gap: "14px",
        height: "48px",
        paddingInline: "14px",
    },
    create: {
        alignItems: "center",
        borderColor: asana.chromeMuted,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "inline-flex",
        fontSize: "13px",
        gap: "6px",
        height: "28px",
        paddingInlineEnd: "12px",
        paddingInlineStart: "4px",
    },
    createMark: {
        alignItems: "center",
        backgroundColor: asana.coral,
        borderRadius: "50%",
        display: "inline-flex",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    search: {
        alignItems: "center",
        backgroundColor: asana.chromeField,
        borderRadius: "999px",
        color: asana.chromeMuted,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        height: "30px",
        marginInline: "auto",
        maxWidth: "460px",
        paddingInline: "12px",
        width: "45%",
    },
    topEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: asana.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "28px",
        justifyContent: "center",
        width: "28px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "200px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // sidebar
    side: {
        color: asana.chromeInk,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        fontSize: "13px",
        gap: "1px",
        paddingBlock: "8px",
        paddingInline: "10px",
    },
    link: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        gap: "10px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    linkOn: {
        backgroundColor: asana.chromeOn,
    },
    group: {
        color: asana.chromeMuted,
        fontSize: "12px",
        paddingBlockEnd: "4px",
        paddingBlockStart: "14px",
        paddingInline: "8px",
    },
    swatch: {
        backgroundColor: asana.coral,
        borderRadius: "4px",
        flexShrink: 0,
        height: "12px",
        width: "12px",
    },
    swatchBlue: {
        backgroundColor: asana.blue,
    },

    // the project
    main: {
        backgroundColor: asana.surface,
        borderStartStartRadius: { default: "8px", [site.narrow]: "0" },
        display: "grid",
        gridTemplateRows: "auto auto auto minmax(0, 1fr)",
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        color: asana.muted,
        display: "flex",
        gap: "8px",
        paddingBlockStart: "14px",
        paddingInline: "20px",
    },
    tile: {
        alignItems: "center",
        backgroundColor: asana.coral,
        borderRadius: "8px",
        color: asana.onAccent,
        display: "inline-flex",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    title: {
        color: asana.ink,
        fontSize: "20px",
        fontWeight: 500,
        whiteSpace: "nowrap",
    },
    status: {
        alignItems: "center",
        borderColor: asana.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: asana.ink,
        display: { default: "inline-flex", [site.compact]: "none" },
        fontSize: "12px",
        gap: "6px",
        paddingBlock: "3px",
        paddingInline: "10px",
    },
    statusDot: {
        backgroundColor: asana.onTrack,
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "4px",
        marginInlineStart: "auto",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: asana.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },
    faceFlorian: {
        backgroundColor: site.avatar,
    },
    faceAgent: {
        backgroundColor: site.agent,
    },
    share: {
        alignItems: "center",
        backgroundColor: asana.blue,
        borderRadius: "6px",
        color: asana.onAccent,
        display: "inline-flex",
        gap: "6px",
        marginInlineStart: "8px",
        paddingBlock: "5px",
        paddingInline: "10px",
    },
    tabs: {
        borderBottomColor: asana.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: asana.muted,
        display: "flex",
        gap: "18px",
        overflow: "hidden",
        paddingBlockStart: "12px",
        paddingInline: "20px",
        whiteSpace: "nowrap",
    },
    tab: {
        paddingBlockEnd: "10px",
    },
    tabOn: {
        boxShadow: `inset 0 -2px 0 ${asana.ink}`,
        color: asana.ink,
        fontWeight: 500,
    },
    toolbar: {
        alignItems: "center",
        display: "flex",
        paddingBlock: "10px",
        paddingInline: "20px",
    },
    add: {
        alignItems: "center",
        backgroundColor: asana.blue,
        borderRadius: "6px",
        color: asana.onAccent,
        display: "inline-flex",
        fontSize: "13px",
        gap: "6px",
        height: "30px",
        paddingInlineStart: "10px",
    },
    addCaret: {
        alignItems: "center",
        borderInlineStartColor: asana.chromeOn,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "inline-flex",
        height: "100%",
        paddingInline: "6px",
    },
    toolbarEnd: {
        color: asana.muted,
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    tool: {
        alignItems: "center",
        display: "inline-flex",
        fontSize: "13px",
        gap: "6px",
    },
    wide: {
        display: { default: "flex", [site.compact]: "none" },
    },
    mid: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    table: {
        alignContent: "start",
        display: "grid",
        overflow: "hidden",
        paddingInline: "20px",
    },
    row: {
        borderBottomColor: asana.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) 130px 90px 100px",
            [site.compact]: "minmax(0, 1fr) 120px 80px",
            [site.narrow]: "minmax(0, 1fr) 80px",
        },
    },
    columns: {
        color: asana.muted,
        fontSize: "12px",
    },
    cell: {
        alignItems: "center",
        borderInlineEndColor: asana.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        display: "flex",
        gap: "6px",
        minWidth: 0,
        paddingBlock: "8px",
        paddingInline: "8px",
    },
    section: {
        alignItems: "center",
        display: "flex",
        fontSize: "15px",
        fontWeight: 600,
        gap: "6px",
        paddingBlockEnd: "6px",
        paddingBlockStart: "14px",
    },
    name: {
        gap: "10px",
        paddingInlineStart: "22px",
    },
    check: {
        alignItems: "center",
        borderColor: asana.muted,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "1px",
        color: asana.muted,
        display: "inline-flex",
        flexShrink: 0,
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    checkDone: {
        backgroundColor: asana.done,
        borderColor: asana.done,
        color: asana.onAccent,
    },
    taskName: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    owner: {
        alignItems: "center",
        borderRadius: "50%",
        color: asana.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "10px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    due: {
        color: asana.muted,
        fontSize: "13px",
    },
    pill: {
        borderRadius: "999px",
        color: asana.pillInk,
        fontSize: "12px",
        paddingBlock: "1px",
        paddingInline: "8px",
    },
    high: {
        backgroundColor: asana.high,
    },
    medium: {
        backgroundColor: asana.medium,
    },
    low: {
        backgroundColor: asana.low,
    },
    addRow: {
        color: asana.muted,
        fontSize: "13px",
        paddingBlock: "8px",
        paddingInlineStart: "48px",
    },
});
