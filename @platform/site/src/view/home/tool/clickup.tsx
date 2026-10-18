import * as style from "@destack/style";

import { tasks, type TaskState } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { clickup } from "./clickup.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    plus: ["M12 5v14", "M5 12h14"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
    home: ["M4 11 12 4l8 7v9h-5v-6H9v6H4z"],
    inbox: ["M4 13h4.5l1.5 3h4l1.5-3H20", "M6 5h12l2 8v6H4v-6z"],
    documents: ["M6 3h9l4 4v14H6z", "M14 3v5h5"],
    dashboard: ["M4 4h7v9H4z", "M13 4h7v5h-7z", "M13 11h7v9h-7z", "M4 15h7v5H4z"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    folder: ["M4 6a1 1 0 0 1 1-1h5l2 2h7a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"],
    list: ["M8 6h12", "M8 12h12", "M8 18h12", "M4 6h.01", "M4 12h.01", "M4 18h.01"],
    board: ["M4 5h4v14H4z", "M10 5h4v9h-4z", "M16 5h4v11h-4z"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    caret: ["m7 10 5 5 5-5"],
    flag: ["M6 21V4", "M6 4h11l-2 4 2 4H6"],
    group: ["M4 5h16v5H4z", "M4 14h16v5H4z"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    people: ["M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z", "M3 20a6 6 0 0 1 12 0"],
} satisfies Record<string, readonly string[]>;

/** The statuses the list groups by: the state, its name and colour. */
const statuses: readonly (readonly [state: TaskState, name: string, tint: string])[] = [
    ["started", "In progress", "#5f55ee"],
    ["review", "Review", "#f8ae00"],
    ["todo", "To do", "#87909e"],
    ["done", "Complete", "#008844"],
];

/** The priority of each launch task, in its order. */
const priorities: readonly ("urgent" | "high" | "normal" | "low")[] = [
    "high",
    "urgent",
    "high",
    "normal",
    "low",
    "normal",
];

/** Draw the launch tasks as the list of a space's folder, grouped by status. */
export function ClickUp() {
    return (
        <div data-component="ClickUpPage" {...style.attrs(styles.page)}>
            {/* global top bar */}
            <header {...style.attrs(styles.bar)}>
                <Favicon icon="clickup" tint="#7b68ee" size={20} />
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={14} />
                    Search
                </span>
                <span {...style.attrs(styles.barEnd)}>
                    <span {...style.attrs(styles.newButton)}>
                        <Mark name="plus" size={13} />
                        New
                    </span>
                    <Mark name="bell" size={16} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* sidebar */}
                <nav {...style.attrs(styles.side)}>
                    <b {...style.attrs(styles.workspace)}>
                        <span {...style.attrs(styles.workspaceMark)}>S</span>
                        Symbol
                        <Mark name="caret" size={12} />
                    </b>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="home" size={15} />
                        Home
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="inbox" size={15} />
                        Inbox
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="documents" size={15} />
                        Docs
                    </span>
                    <span {...style.attrs(styles.link)}>
                        <Mark name="dashboard" size={15} />
                        Dashboards
                    </span>
                    <span {...style.attrs(styles.group)}>Spaces</span>
                    <span {...style.attrs(styles.link)}>
                        <span {...style.attrs(styles.spaceMark)}>D</span>
                        Destack
                    </span>
                    <span {...style.attrs(styles.link, styles.indent)}>
                        <Mark name="folder" size={15} />
                        Launch
                    </span>
                    <span {...style.attrs(styles.link, styles.indentTwice, styles.linkOn)}>
                        <Mark name="list" size={15} />
                        Tasks
                        <span {...style.attrs(styles.count)}>6</span>
                    </span>
                </nav>

                {/* the list */}
                <main {...style.attrs(styles.main)}>
                    <span {...style.attrs(styles.crumbs)}>
                        <span {...style.attrs(styles.spaceMark)}>D</span>
                        Destack / Launch /<b {...style.attrs(styles.crumbOn)}>Tasks</b>
                        <Mark name="star" size={14} />
                    </span>
                    <span {...style.attrs(styles.views)}>
                        <span {...style.attrs(styles.view, styles.viewOn)}>
                            <Mark name="list" size={14} />
                            List
                        </span>
                        <span {...style.attrs(styles.view)}>
                            <Mark name="board" size={14} />
                            Board
                        </span>
                        <span {...style.attrs(styles.view, styles.wide)}>
                            <Mark name="calendar" size={14} />
                            Calendar
                        </span>
                        <span {...style.attrs(styles.view)}>
                            <Mark name="plus" size={14} />
                            View
                        </span>
                    </span>
                    <span {...style.attrs(styles.toolbar)}>
                        <span {...style.attrs(styles.chip)}>
                            <Mark name="group" size={13} />
                            Group: Status
                        </span>
                        <span {...style.attrs(styles.chip, styles.wide)}>
                            <Mark name="filter" size={13} />
                            Filter
                        </span>
                        <span {...style.attrs(styles.chip, styles.wide)}>
                            <Mark name="people" size={13} />
                            Assignee
                        </span>
                        <span {...style.attrs(styles.toolbarEnd)}>
                            <Mark name="search" size={14} />
                        </span>
                    </span>

                    <div {...style.attrs(styles.groups)}>
                        {statuses.map(([state, name, tint]) => (
                            <section {...style.attrs(styles.status)}>
                                <span {...style.attrs(styles.statusHead)}>
                                    <Mark name="caret" size={12} />
                                    <span
                                        style={{ "background-color": tint }}
                                        {...style.attrs(styles.pill)}
                                    >
                                        {name}
                                    </span>
                                    <span {...style.attrs(styles.muted)}>
                                        {tasks.filter((task) => task[2] === state).length}
                                    </span>
                                    <span {...style.attrs(styles.addTask)}>
                                        <Mark name="plus" size={12} />
                                        Add Task
                                    </span>
                                </span>
                                <span {...style.attrs(styles.row, styles.columns)}>
                                    <span>Name</span>
                                    <span>Assignee</span>
                                    <span {...style.attrs(styles.wide)}>Due date</span>
                                    <span>Priority</span>
                                </span>
                                {tasks.map(([, title, taskState, owner, ownerTint, due], index) =>
                                    taskState === state ? (
                                        <span {...style.attrs(styles.row)}>
                                            <span {...style.attrs(styles.name)}>
                                                <span
                                                    style={{ "border-color": tint }}
                                                    {...style.attrs(styles.ring)}
                                                />
                                                <span {...style.attrs(styles.title)}>{title}</span>
                                            </span>
                                            <span>
                                                <span
                                                    style={{ "background-color": ownerTint }}
                                                    {...style.attrs(styles.owner)}
                                                >
                                                    {owner}
                                                </span>
                                            </span>
                                            <span {...style.attrs(styles.muted, styles.wide)}>
                                                {due}
                                            </span>
                                            <span {...style.attrs(styles.priority)}>
                                                <span
                                                    {...style.attrs(
                                                        styles.flag,
                                                        priorities[index] === "urgent"
                                                            ? styles.urgent
                                                            : priorities[index] === "high"
                                                              ? styles.high
                                                              : priorities[index] === "normal"
                                                                ? styles.normal
                                                                : styles.low,
                                                    )}
                                                >
                                                    <Mark name="flag" size={14} />
                                                </span>
                                                <span {...style.attrs(styles.wide)}>
                                                    {priorities[index]}
                                                </span>
                                            </span>
                                        </span>
                                    ) : undefined,
                                )}
                            </section>
                        ))}
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

/** The ClickUp page styles. */
const styles = style.create({
    page: {
        backgroundColor: clickup.bar,
        color: clickup.ink,
        display: "grid",
        fontFamily: clickup.face,
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
        strokeWidth: 1.8,
    },
    muted: {
        color: clickup.muted,
    },

    // global top bar
    bar: {
        alignItems: "center",
        color: clickup.barMuted,
        display: "flex",
        gap: "12px",
        height: "40px",
        paddingInline: "12px",
    },
    search: {
        alignItems: "center",
        backgroundColor: clickup.barField,
        borderRadius: "6px",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        height: "26px",
        marginInline: "auto",
        maxWidth: "420px",
        paddingInline: "10px",
        width: "45%",
    },
    barEnd: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
    },
    newButton: {
        alignItems: "center",
        backgroundColor: clickup.purple,
        borderRadius: "6px",
        color: clickup.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "4px",
        height: "26px",
        paddingInline: "10px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: clickup.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    body: {
        backgroundColor: clickup.surface,
        display: "grid",
        gridTemplateColumns: { default: "200px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // sidebar
    side: {
        backgroundColor: clickup.side,
        borderInlineEndColor: clickup.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: clickup.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "1px",
        paddingBlock: "10px",
        paddingInline: "8px",
    },
    workspace: {
        alignItems: "center",
        color: clickup.ink,
        display: "flex",
        gap: "8px",
        paddingBlockEnd: "10px",
        paddingInline: "6px",
    },
    workspaceMark: {
        alignItems: "center",
        backgroundColor: clickup.purple,
        borderRadius: "6px",
        color: clickup.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    link: {
        alignItems: "center",
        borderRadius: "6px",
        display: "flex",
        gap: "8px",
        paddingBlock: "5px",
        paddingInline: "8px",
    },
    linkOn: {
        backgroundColor: clickup.selected,
        color: clickup.purple,
        fontWeight: 600,
    },
    indent: {
        paddingInlineStart: "22px",
    },
    indentTwice: {
        paddingInlineStart: "36px",
    },
    count: {
        color: clickup.muted,
        fontWeight: 400,
        marginInlineStart: "auto",
    },
    group: {
        fontSize: "11px",
        fontWeight: 600,
        paddingBlockEnd: "4px",
        paddingBlockStart: "12px",
        paddingInline: "8px",
    },
    spaceMark: {
        alignItems: "center",
        backgroundColor: site.space,
        borderRadius: "4px",
        color: clickup.onAccent,
        display: "inline-flex",
        fontSize: "10px",
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },

    // the list
    main: {
        alignContent: "start",
        display: "grid",
        gridTemplateRows: "auto auto auto minmax(0, 1fr)",
        minWidth: 0,
        overflow: "hidden",
    },
    crumbs: {
        alignItems: "center",
        color: clickup.muted,
        display: "flex",
        gap: "6px",
        paddingBlock: "10px",
        paddingInline: "16px",
        whiteSpace: "nowrap",
    },
    crumbOn: {
        color: clickup.ink,
    },
    views: {
        borderBottomColor: clickup.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: clickup.muted,
        display: "flex",
        gap: "4px",
        paddingInline: "12px",
    },
    view: {
        alignItems: "center",
        display: "inline-flex",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "8px",
    },
    viewOn: {
        boxShadow: `inset 0 -2px 0 ${clickup.purple}`,
        color: clickup.ink,
        fontWeight: 600,
    },
    wide: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },
    toolbar: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "16px",
    },
    chip: {
        alignItems: "center",
        borderColor: clickup.line,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: clickup.ink,
        display: "inline-flex",
        gap: "6px",
        height: "26px",
        paddingInline: "8px",
    },
    toolbarEnd: {
        color: clickup.muted,
        display: "flex",
        marginInlineStart: "auto",
    },
    groups: {
        alignContent: "start",
        display: "grid",
        gap: "10px",
        overflow: "hidden",
        paddingInline: "16px",
    },
    status: {
        display: "grid",
    },
    statusHead: {
        alignItems: "center",
        color: clickup.muted,
        display: "flex",
        gap: "8px",
        paddingBlock: "6px",
    },
    pill: {
        borderRadius: "4px",
        color: clickup.onAccent,
        fontSize: "11px",
        fontWeight: 600,
        paddingBlock: "2px",
        paddingInline: "8px",
        textTransform: "uppercase",
    },
    addTask: {
        alignItems: "center",
        display: "inline-flex",
        fontSize: "12px",
        gap: "4px",
    },
    row: {
        alignItems: "center",
        borderBottomColor: clickup.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        gap: "10px",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) 70px 70px 80px",
            [site.compact]: "minmax(0, 1fr) 70px 50px",
        },
        paddingBlock: "7px",
        paddingInlineStart: "20px",
    },
    columns: {
        color: clickup.muted,
        fontSize: "11px",
        paddingBlock: "4px",
    },
    name: {
        alignItems: "center",
        display: "flex",
        gap: "10px",
        minWidth: 0,
    },
    ring: {
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "3px",
        boxSizing: "border-box",
        flexShrink: 0,
        height: "13px",
        width: "13px",
    },
    title: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    owner: {
        alignItems: "center",
        borderRadius: "50%",
        color: clickup.onAccent,
        display: "inline-flex",
        fontSize: "10px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    priority: {
        alignItems: "center",
        color: clickup.muted,
        display: "flex",
        gap: "4px",
        textTransform: "capitalize",
    },
    flag: {
        display: "flex",
    },
    urgent: {
        color: clickup.urgent,
    },
    high: {
        color: clickup.high,
    },
    normal: {
        color: clickup.normal,
    },
    low: {
        color: clickup.low,
    },
});
