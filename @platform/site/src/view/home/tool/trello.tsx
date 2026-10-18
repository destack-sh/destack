import * as style from "@destack/style";

import { tasks, type TaskState } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { trello } from "./trello.stylex";

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
    caret: ["m7 10 5 5 5-5"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    people: [
        "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
        "M3 20a6 6 0 0 1 12 0",
        "M16 11a3 3 0 1 0 0-6",
        "M17.5 14.5A5 5 0 0 1 21 19.5",
    ],
    board: ["M4 5h4v14H4z", "M10 5h4v9h-4z", "M16 5h4v11h-4z"],
    bolt: ["M13 3 5 14h6l-1 7 8-11h-6z"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    addPerson: [
        "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
        "M3 20a6 6 0 0 1 12 0",
        "M19 8v6",
        "M16 11h6",
    ],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    eye: [
        "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z",
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
    ],
    clock: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5l3 2"],
    text: ["M4 6h16", "M4 12h16", "M4 18h10"],
    comment: ["M5 5h14v11H9l-4 4z"],
    checklist: ["M4 4h16v16H4z", "m8 12 3 3 5-6"],
    plus: ["M12 5v14", "M5 12h14"],
    template: ["M4 4h16v6H4z", "M4 14h7v6H4z", "M15 14h5v6h-5z"],
} satisfies Record<string, readonly string[]>;

/** The board's lists in the order the board shows them, with their names. */
const lists: readonly (readonly [state: TaskState, name: string])[] = [
    ["todo", "To do"],
    ["started", "Doing"],
    ["review", "Review"],
    ["done", "Done"],
];

/** The checklist progress of each launch task, in its order. */
const checklists: readonly string[] = ["4/4", "1/3", "2/5", "0/2", "0/4", "0/3"];

/** Draw the launch tasks as cards on the team's board, one list per status. */
export function Trello() {
    return (
        <div data-component="TrelloPage" {...style.attrs(styles.page)}>
            {/* app header */}
            <header {...style.attrs(styles.header)}>
                <Mark name="switcher" size={16} />
                <span {...style.attrs(styles.logo)}>
                    <Favicon icon="trello" tint="#ffffff" size={16} />
                    Trello
                </span>
                <span {...style.attrs(styles.nav)}>
                    <span {...style.attrs(styles.navItem)}>
                        Workspaces
                        <Mark name="caret" size={12} />
                    </span>
                    <span {...style.attrs(styles.navItem)}>
                        Recent
                        <Mark name="caret" size={12} />
                    </span>
                    <span {...style.attrs(styles.navItem)}>
                        Starred
                        <Mark name="caret" size={12} />
                    </span>
                </span>
                <span {...style.attrs(styles.create)}>Create</span>
                <span {...style.attrs(styles.headerEnd)}>
                    <span {...style.attrs(styles.search)}>
                        <Mark name="search" size={14} />
                        Search
                    </span>
                    <Mark name="bell" size={16} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            {/* board header */}
            <div {...style.attrs(styles.strip)}>
                <b {...style.attrs(styles.title)}>Destack launch</b>
                <Mark name="star" size={15} />
                <Mark name="people" size={15} />
                <span {...style.attrs(styles.view)}>
                    <Mark name="board" size={14} />
                    Board
                </span>
                <span {...style.attrs(styles.stripEnd)}>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="bolt" size={15} />
                    </span>
                    <span {...style.attrs(styles.wide)}>
                        <Mark name="filter" size={15} />
                    </span>
                    <span {...style.attrs(styles.members)}>
                        <span {...style.attrs(styles.member, styles.memberFlorian)}>F</span>
                        <span {...style.attrs(styles.member, styles.memberAgent)}>A</span>
                    </span>
                    <span {...style.attrs(styles.share)}>
                        <Mark name="addPerson" size={14} />
                        Share
                    </span>
                    <Mark name="more" size={15} />
                </span>
            </div>

            {/* lists */}
            <div {...style.attrs(styles.board)}>
                {lists.map(([state, name]) => (
                    <section {...style.attrs(styles.list)}>
                        <b {...style.attrs(styles.listName)}>
                            {name}
                            <Mark name="more" size={15} />
                        </b>
                        {tasks.map(([, title, taskState, owner, tint, due], index) =>
                            taskState === state ? (
                                <article {...style.attrs(styles.card)}>
                                    <span {...style.attrs(styles.labels)}>
                                        <span {...style.attrs(styles.label, styles.green)} />
                                        {state === "review" ? (
                                            <span {...style.attrs(styles.label, styles.yellow)} />
                                        ) : undefined}
                                        {title.includes("waitlist") || title.includes("signups") ? (
                                            <span {...style.attrs(styles.label, styles.purple)} />
                                        ) : undefined}
                                    </span>
                                    <span {...style.attrs(styles.cardTitle)}>{title}</span>
                                    <span {...style.attrs(styles.badges)}>
                                        {index === 1 ? <Mark name="eye" size={14} /> : undefined}
                                        <span
                                            {...style.attrs(
                                                styles.badge,
                                                state === "done" && styles.badgeDone,
                                                state === "review" && styles.badgeSoon,
                                            )}
                                        >
                                            <Mark name="clock" size={13} />
                                            {due}
                                        </span>
                                        <Mark name="text" size={14} />
                                        {index % 2 === 0 ? (
                                            <span {...style.attrs(styles.badge)}>
                                                <Mark name="comment" size={13} />
                                                {index + 1}
                                            </span>
                                        ) : undefined}
                                        <span
                                            {...style.attrs(
                                                styles.badge,
                                                state === "done" && styles.badgeDone,
                                            )}
                                        >
                                            <Mark name="checklist" size={13} />
                                            {checklists[index]}
                                        </span>
                                        <span
                                            style={{ "background-color": tint }}
                                            {...style.attrs(styles.owner)}
                                        >
                                            {owner}
                                        </span>
                                    </span>
                                </article>
                            ) : undefined,
                        )}
                        <span {...style.attrs(styles.add)}>
                            <Mark name="plus" size={14} />
                            Add a card
                            <span {...style.attrs(styles.addEnd)}>
                                <Mark name="template" size={14} />
                            </span>
                        </span>
                    </section>
                ))}
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

/** The Trello page styles. */
const styles = style.create({
    page: {
        backgroundImage: trello.backdrop,
        color: trello.ink,
        display: "grid",
        fontFamily: trello.face,
        fontSize: "14px",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto minmax(0, 1fr)",
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

    // app header
    header: {
        alignItems: "center",
        backgroundColor: trello.header,
        color: trello.onBackdrop,
        display: "flex",
        gap: "10px",
        height: "48px",
        paddingInline: "10px",
    },
    logo: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        fontWeight: 700,
        gap: "6px",
    },
    nav: {
        display: { default: "flex", [site.compact]: "none" },
        gap: "4px",
    },
    navItem: {
        alignItems: "center",
        borderRadius: "4px",
        display: "flex",
        fontWeight: 500,
        gap: "4px",
        paddingBlock: "6px",
        paddingInline: "10px",
    },
    create: {
        backgroundColor: trello.glass,
        borderRadius: "4px",
        fontWeight: 500,
        paddingBlock: "6px",
        paddingInline: "12px",
    },
    headerEnd: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        marginInlineStart: "auto",
    },
    search: {
        alignItems: "center",
        backgroundColor: trello.glass,
        borderRadius: "4px",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "6px",
        height: "30px",
        paddingInline: "8px",
        width: "160px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        width: "26px",
    },

    // board header
    strip: {
        alignItems: "center",
        backgroundColor: trello.strip,
        color: trello.onBackdrop,
        display: "flex",
        gap: "12px",
        height: "52px",
        paddingInline: "14px",
    },
    title: {
        fontSize: "18px",
        whiteSpace: "nowrap",
    },
    view: {
        alignItems: "center",
        backgroundColor: trello.glass,
        borderRadius: "4px",
        display: "inline-flex",
        fontWeight: 500,
        gap: "6px",
        paddingBlock: "5px",
        paddingInline: "10px",
    },
    stripEnd: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        marginInlineStart: "auto",
    },
    wide: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    members: {
        display: "flex",
    },
    member: {
        alignItems: "center",
        borderRadius: "50%",
        display: "inline-flex",
        fontSize: "11px",
        height: "26px",
        justifyContent: "center",
        marginInlineEnd: "-4px",
        width: "26px",
    },
    memberFlorian: {
        backgroundColor: site.avatar,
    },
    memberAgent: {
        backgroundColor: site.agent,
    },
    share: {
        alignItems: "center",
        backgroundColor: trello.button,
        borderRadius: "4px",
        color: trello.buttonInk,
        display: "inline-flex",
        fontWeight: 500,
        gap: "6px",
        marginInlineStart: "4px",
        paddingBlock: "6px",
        paddingInline: "10px",
    },

    // lists
    board: {
        alignItems: "start",
        display: "grid",
        gap: "12px",
        gridAutoColumns: { default: "272px", [site.narrow]: "80%" },
        gridAutoFlow: "column",
        overflow: "hidden",
        padding: "12px",
    },
    list: {
        backgroundColor: trello.list,
        borderRadius: "12px",
        display: "grid",
        gap: "8px",
        padding: "8px",
    },
    listName: {
        alignItems: "center",
        display: "flex",
        justifyContent: "space-between",
        paddingBlock: "6px",
        paddingInline: "8px",
    },
    card: {
        backgroundColor: trello.card,
        borderRadius: "8px",
        boxShadow: trello.lift,
        display: "grid",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    labels: {
        display: "flex",
        gap: "4px",
    },
    label: {
        borderRadius: "4px",
        height: "8px",
        width: "40px",
    },
    green: {
        backgroundColor: trello.green,
    },
    yellow: {
        backgroundColor: trello.yellow,
    },
    purple: {
        backgroundColor: trello.purple,
    },
    cardTitle: {
        lineHeight: 1.35,
    },
    badges: {
        alignItems: "center",
        color: trello.muted,
        display: "flex",
        flexWrap: "wrap",
        fontSize: "12px",
        gap: "8px",
    },
    badge: {
        alignItems: "center",
        borderRadius: "3px",
        display: "inline-flex",
        gap: "4px",
        paddingBlock: "1px",
        paddingInline: "3px",
    },
    badgeDone: {
        backgroundColor: trello.done,
        color: trello.onBackdrop,
    },
    badgeSoon: {
        backgroundColor: trello.soon,
        color: trello.ink,
    },
    owner: {
        alignItems: "center",
        borderRadius: "50%",
        color: trello.onBackdrop,
        display: "inline-flex",
        fontSize: "10px",
        height: "24px",
        justifyContent: "center",
        marginInlineStart: "auto",
        width: "24px",
    },
    add: {
        alignItems: "center",
        color: trello.muted,
        display: "flex",
        fontWeight: 500,
        gap: "6px",
        paddingBlock: "4px",
        paddingInline: "8px",
    },
    addEnd: {
        display: "flex",
        marginInlineStart: "auto",
    },
});
