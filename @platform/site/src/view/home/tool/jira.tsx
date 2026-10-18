import * as style from "@destack/style";

import { tasks, type TaskState } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { jira } from "./jira.stylex";

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
    summary: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 3v9h9"],
    timeline: ["M4 6h9", "M8 12h10", "M6 18h8"],
    board: ["M4 5h4v14H4z", "M10 5h4v9h-4z", "M16 5h4v11h-4z"],
    list: ["M8 6h12", "M8 12h12", "M8 18h12", "M4 6h.01", "M4 12h.01", "M4 18h.01"],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    caret: ["m7 10 5 5 5-5"],
    up: ["m6 15 6-6 6 6", "m6 19 6-6 6 6"],
    insights: ["M5 20V10", "M12 20V4", "M19 20v-7"],
    check: ["m6 12.5 4 4 8-9"],
} satisfies Record<string, readonly string[]>;

/** The board's columns in the order a board shows them, with their names. */
const columns: readonly (readonly [state: TaskState, name: string])[] = [
    ["todo", "To Do"],
    ["started", "In Progress"],
    ["review", "In Review"],
    ["done", "Done"],
];

/** The project's views in the sidebar, the board chosen. */
const views: readonly (readonly [name: string, glyph: keyof typeof glyphs])[] = [
    ["Summary", "summary"],
    ["Timeline", "timeline"],
    ["Board", "board"],
    ["List", "list"],
    ["Calendar", "calendar"],
];

/** Draw the launch tasks as the project's board, one column per status. */
export function Jira() {
    return (
        <div data-component="JiraPage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.top)}>
                <Mark name="sidebar" size={18} />
                <Mark name="switcher" size={18} />
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="jira" tint="#1868db" size={22} />
                    Jira
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
                    <span {...style.attrs(styles.group)}>Projects</span>
                    <span {...style.attrs(styles.project)}>
                        <span {...style.attrs(styles.projectMark)}>D</span>
                        <b>Destack</b>
                    </span>
                    {views.map(([name, glyph]) => (
                        <span {...style.attrs(styles.view, name === "Board" && styles.viewOn)}>
                            <Mark name={glyph} size={15} />
                            {name}
                        </span>
                    ))}
                </nav>

                {/* the board */}
                <main {...style.attrs(styles.main)}>
                    <span {...style.attrs(styles.crumbs)}>Projects / Destack</span>
                    <span {...style.attrs(styles.titleRow)}>
                        <b {...style.attrs(styles.title)}>DST board</b>
                        <Mark name="star" size={16} />
                        <Mark name="more" size={16} />
                    </span>
                    <span {...style.attrs(styles.toolbar)}>
                        <span {...style.attrs(styles.boardSearch)}>
                            <Mark name="search" size={14} />
                            Search board
                        </span>
                        <span {...style.attrs(styles.faces)}>
                            <span {...style.attrs(styles.face, styles.faceFlorian)}>F</span>
                            <span {...style.attrs(styles.face, styles.faceAgent)}>A</span>
                        </span>
                        <span {...style.attrs(styles.filter)}>
                            Epic
                            <Mark name="caret" size={12} />
                        </span>
                        <span {...style.attrs(styles.filter, styles.wide)}>
                            Type
                            <Mark name="caret" size={12} />
                        </span>
                        <span {...style.attrs(styles.toolbarEnd)}>
                            <span {...style.attrs(styles.filter, styles.wide)}>
                                <Mark name="insights" size={14} />
                                Insights
                            </span>
                        </span>
                    </span>

                    <div {...style.attrs(styles.board)}>
                        {columns.map(([state, name]) => (
                            <section {...style.attrs(styles.column)}>
                                <span {...style.attrs(styles.columnName)}>
                                    {name}
                                    <span>{tasks.filter((task) => task[2] === state).length}</span>
                                    {state === "done" ? (
                                        <span {...style.attrs(styles.doneMark)}>
                                            <Mark name="check" size={13} />
                                        </span>
                                    ) : undefined}
                                </span>
                                {tasks
                                    .filter((task) => task[2] === state)
                                    .map(([id, title, , owner, tint], index) => (
                                        <article {...style.attrs(styles.card)}>
                                            <span {...style.attrs(styles.cardTitle)}>{title}</span>
                                            <span {...style.attrs(styles.epic)}>Launch</span>
                                            <span {...style.attrs(styles.cardFoot)}>
                                                <span
                                                    {...style.attrs(
                                                        styles.type,
                                                        index % 2 === 0
                                                            ? styles.story
                                                            : styles.task,
                                                    )}
                                                >
                                                    <Mark name="check" size={10} />
                                                </span>
                                                <span
                                                    {...style.attrs(
                                                        styles.key,
                                                        state === "done" && styles.keyDone,
                                                    )}
                                                >
                                                    {id}
                                                </span>
                                                <span {...style.attrs(styles.cardEnd)}>
                                                    <span {...style.attrs(styles.priority)}>
                                                        <Mark name="up" size={14} />
                                                    </span>
                                                    <span {...style.attrs(styles.points)}>
                                                        {(index % 3) + 1}
                                                    </span>
                                                    <span
                                                        style={{ "background-color": tint }}
                                                        {...style.attrs(styles.owner)}
                                                    >
                                                        {owner}
                                                    </span>
                                                </span>
                                            </span>
                                        </article>
                                    ))}
                                <span {...style.attrs(styles.add)}>
                                    <Mark name="plus" size={14} />
                                    Create
                                </span>
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

/** The Jira page styles. */
const styles = style.create({
    page: {
        backgroundColor: jira.surface,
        color: jira.ink,
        display: "grid",
        fontFamily: jira.face,
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
        borderBottomColor: jira.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: jira.muted,
        display: "flex",
        gap: "12px",
        height: "48px",
        paddingInline: "12px",
    },
    brand: {
        alignItems: "center",
        color: jira.ink,
        display: "flex",
        fontSize: "16px",
        fontWeight: 600,
        gap: "6px",
    },
    search: {
        alignItems: "center",
        borderColor: jira.line,
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
        backgroundColor: jira.blue,
        borderRadius: "4px",
        color: jira.onAccent,
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
        color: jira.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "220px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // sidebar
    side: {
        backgroundColor: jira.side,
        borderInlineEndColor: jira.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: jira.muted,
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
    group: {
        color: jira.faint,
        fontSize: "12px",
        fontWeight: 600,
        paddingBlockEnd: "4px",
        paddingBlockStart: "14px",
        paddingInline: "8px",
    },
    project: {
        alignItems: "center",
        color: jira.ink,
        display: "flex",
        gap: "8px",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    projectMark: {
        alignItems: "center",
        backgroundColor: site.space,
        borderRadius: "4px",
        color: jira.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        fontWeight: 700,
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    view: {
        alignItems: "center",
        borderRadius: "4px",
        display: "flex",
        gap: "10px",
        paddingBlock: "6px",
        paddingInlineEnd: "8px",
        paddingInlineStart: "22px",
    },
    viewOn: {
        backgroundColor: jira.selected,
        color: jira.blue,
        fontWeight: 500,
    },

    // the board
    main: {
        alignContent: "start",
        display: "grid",
        gridTemplateRows: "auto auto auto minmax(0, 1fr)",
        minWidth: 0,
        overflow: "hidden",
        paddingInline: { default: "24px", [site.narrow]: "14px" },
    },
    crumbs: {
        color: jira.faint,
        fontSize: "13px",
        paddingBlockStart: "16px",
    },
    titleRow: {
        alignItems: "center",
        color: jira.muted,
        display: "flex",
        gap: "10px",
        paddingBlockEnd: "12px",
        paddingBlockStart: "6px",
    },
    title: {
        color: jira.ink,
        fontSize: "22px",
        fontWeight: 650,
    },
    toolbar: {
        alignItems: "center",
        color: jira.muted,
        display: "flex",
        gap: "10px",
        paddingBlockEnd: "14px",
    },
    boardSearch: {
        alignItems: "center",
        borderColor: jira.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "flex",
        gap: "6px",
        height: "30px",
        paddingInline: "8px",
        width: "150px",
    },
    faces: {
        display: "flex",
    },
    face: {
        alignItems: "center",
        borderColor: jira.surface,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        color: jira.onAccent,
        display: "inline-flex",
        fontSize: "11px",
        height: "28px",
        justifyContent: "center",
        marginInlineEnd: "-6px",
        width: "28px",
    },
    faceFlorian: {
        backgroundColor: site.avatar,
    },
    faceAgent: {
        backgroundColor: site.agent,
    },
    filter: {
        alignItems: "center",
        borderRadius: "4px",
        color: jira.ink,
        display: "inline-flex",
        gap: "4px",
        height: "30px",
        paddingInline: "8px",
    },
    wide: {
        display: { default: "inline-flex", [site.compact]: "none" },
    },
    toolbarEnd: {
        display: "flex",
        marginInlineStart: "auto",
    },
    board: {
        alignItems: "start",
        display: "grid",
        gap: "8px",
        gridAutoColumns: { default: "minmax(170px, 1fr)", [site.narrow]: "75%" },
        gridAutoFlow: "column",
        overflow: "hidden",
    },
    column: {
        backgroundColor: jira.column,
        borderRadius: "8px",
        display: "grid",
        gap: "6px",
        padding: "8px",
    },
    columnName: {
        alignItems: "center",
        color: jira.faint,
        display: "flex",
        fontSize: "12px",
        fontWeight: 600,
        gap: "6px",
        paddingBlock: "4px",
        paddingInline: "4px",
        textTransform: "uppercase",
    },
    doneMark: {
        color: jira.story,
        display: "flex",
    },
    card: {
        backgroundColor: jira.card,
        borderRadius: "6px",
        boxShadow: jira.lift,
        display: "grid",
        gap: "10px",
        padding: "12px",
    },
    cardTitle: {
        lineHeight: 1.35,
    },
    epic: {
        backgroundColor: jira.epic,
        borderRadius: "3px",
        color: jira.epicInk,
        fontSize: "11px",
        fontWeight: 700,
        justifySelf: "start",
        paddingInline: "4px",
        textTransform: "uppercase",
    },
    cardFoot: {
        alignItems: "center",
        color: jira.faint,
        display: "flex",
        fontSize: "12px",
        gap: "6px",
    },
    type: {
        alignItems: "center",
        borderRadius: "3px",
        color: jira.onAccent,
        display: "inline-flex",
        height: "16px",
        justifyContent: "center",
        width: "16px",
    },
    story: {
        backgroundColor: jira.story,
    },
    task: {
        backgroundColor: jira.task,
    },
    key: {
        fontWeight: 600,
        whiteSpace: "nowrap",
    },
    keyDone: {
        textDecorationLine: "line-through",
    },
    cardEnd: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
        marginInlineStart: "auto",
    },
    priority: {
        color: jira.priority,
        display: "flex",
    },
    points: {
        backgroundColor: jira.column,
        borderRadius: "999px",
        color: jira.ink,
        fontSize: "11px",
        minWidth: "18px",
        textAlign: "center",
    },
    owner: {
        alignItems: "center",
        borderRadius: "50%",
        color: jira.onAccent,
        display: "inline-flex",
        fontSize: "10px",
        height: "22px",
        justifyContent: "center",
        width: "22px",
    },
    add: {
        alignItems: "center",
        color: jira.muted,
        display: "flex",
        fontSize: "13px",
        gap: "6px",
        paddingBlock: "4px",
        paddingInline: "6px",
    },
});
