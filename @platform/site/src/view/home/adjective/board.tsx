import { color, font, shadow, stroke } from "@destack/theme/tokens.stylex";
import * as style from "@destack/style";
import type { JSX } from "@destack/view";

import { AppGlyph } from "../figure/app";
import { Fade } from "../figure/fade";
import { Favicon, Glyph } from "../figure/glyph";
import type { Stagger } from "../figure/stagger";
import { ServiceGlyph } from "../figure/tile";
import { palette } from "../../palette.stylex";

/** A mock too narrow for the chat and the board side by side, which then stack. */
const NARROW = "@container (max-width: 959px)";

/** A mock narrower than a phone, whose board stacks its columns and drops what only fits wide. */
const PHONE = "@container (max-width: 479px)";

/** The launch tasks still open: the identifier, the title and the due date. */
const tasks: readonly (readonly [id: string, title: string, due: string])[] = [
    ["DST-14", "Record the demo", "Wed"],
    ["DST-15", "Draft the launch post", "Thu"],
    ["DST-16", "Email the waitlist", "Thu"],
    ["DST-13", "Turn on pricing", "Fri"],
    ["DST-17", "Open signups", "Fri"],
];

/** The latest waitlist signups: the address and when they joined. */
const signups: readonly (readonly [email: string, joined: string])[] = [
    ["maya@northwind.io", "2 min"],
    ["jonas@kettle.dev", "14 min"],
    ["tara@fieldnotes.co", "1 h"],
    ["leo@brightpath.app", "3 h"],
    ["sam@quietly.so", "5 h"],
];

/** The demos booked this week: the day and time, and who booked it. */
const demos: readonly (readonly [time: string, person: string])[] = [
    ["Wed 11:00", "Maya"],
    ["Thu 15:00", "Jonas"],
    ["Fri 09:30", "Tara"],
    ["Fri 11:30", "Ines"],
    ["Fri 16:30", "Leo"],
];

/** The tracker's board of launch issues: each status and its issues. */
const lanes: readonly (readonly [
    lane: string,
    issues: readonly (readonly [id: string, title: string])[],
])[] = [
    [
        "Todo",
        [
            ["DST-13", "Turn on pricing"],
            ["DST-17", "Open signups"],
        ],
    ],
    [
        "In Progress",
        [
            ["DST-14", "Record the demo"],
            ["DST-15", "Draft the launch post"],
        ],
    ],
    ["In Review", [["DST-16", "Email the waitlist"]]],
];

/** What the tracker's filter menu offers: its own issue fields, and nothing from other apps. */
const filters = ["Status", "Assignee", "Labels", "Project", "Cycle", "Priority"];

/** Draw a message of yours as a rounded pill on the right. */
function UserMessage(properties: { children: JSX.Element }) {
    return (
        <p data-component="UserMessage" {...style.attrs(styles.bubble)}>
            {properties.children}
        </p>
    );
}

/** A rented assistant the chat runs in. */
export type Assistant = { name: string; icon: string; tint: string; prompt: string; work: string };

/** The rented assistants the chat turns between while stacked. */
export const assistants: readonly Assistant[] = [
    {
        name: "ChatGPT",
        icon: "openai",
        tint: "#0d0d0d",
        prompt: "Ask anything",
        work: "Worked for",
    },
    {
        name: "Claude",
        icon: "claude",
        tint: "#d97757",
        prompt: "Reply to Claude…",
        work: "Thought for",
    },
];

/** Draw a reply as the chat sets it. */
function Reply(properties: { work: string; time: string; children: JSX.Element }) {
    return (
        <div data-component="AgentMessage" {...style.attrs(styles.reply)}>
            <p data-component="WorkSummary" {...style.attrs(styles.worked)}>
                {properties.work} {properties.time}
                <Glyph size={15} name="forward" />
            </p>
            <div {...style.attrs(styles.replyText)}>{properties.children}</div>
            <p data-component="MessageActions" {...style.attrs(styles.actions)}>
                <Glyph size={15} name="copy" />
                <Glyph size={15} name="like" />
                <Glyph size={15} name="dislike" />
                <Glyph size={15} name="share" />
            </p>
        </div>
    );
}

/** Draw one column of the board: its app's glyph and name, its count, its rows, and their type. */
function Column(properties: {
    icon: string;
    name: string;
    count: string;
    type: string;
    children: JSX.Element;
}) {
    return (
        <section data-component="BoardColumn" {...style.attrs(styles.panel)}>
            <p {...style.attrs(styles.panelHead)}>
                <AppGlyph name={properties.icon} />
                {properties.name}
                <span {...style.attrs(styles.count)}>{properties.count}</span>
            </p>
            <div {...style.attrs(styles.stack)}>{properties.children}</div>
            <p data-component="ColumnSource" {...style.attrs(styles.source)}>
                <span title="Live" {...style.attrs(styles.dot)} />
                <span {...style.attrs(styles.sourceType)}>{properties.type}</span>
            </p>
        </section>
    );
}

/** Draw the board your agent added to Tasks: its new tab, the merged change and its columns. */
function TasksBoard() {
    return (
        <>
            <p {...style.attrs(styles.bar)}>
                <ServiceGlyph icon="tasks" />
                <span {...style.attrs(styles.title)}>Launch tasks</span>
                <span {...style.attrs(styles.tabs)}>
                    <span {...style.attrs(styles.tab)}>List</span>
                    <span {...style.attrs(styles.tab, styles.tabOn)}>
                        Board
                        <span {...style.attrs(styles.new)}>New</span>
                    </span>
                    <span {...style.attrs(styles.tab, styles.wide)}>Calendar</span>
                </span>
                <span title="Live" {...style.attrs(styles.dot, styles.pushed)} />
            </p>
            <div data-component="Board" {...style.attrs(styles.body)}>
                <p data-component="ForgeChange" {...style.attrs(styles.change)}>
                    <Glyph size={15} name="check" />
                    <code {...style.attrs(styles.code)}>tasks/board.tsx</code>
                    <span {...style.attrs(styles.added)}>+38</span>
                    <span {...style.attrs(styles.quiet, styles.wide)}>merged by you in Forge</span>
                    <span {...style.attrs(styles.pushed, styles.button)}>View change</span>
                </p>
                <div {...style.attrs(styles.columns)}>
                    <Column icon="tasks" name="Open tasks" count="5" type="Task">
                        {tasks.map(([id, title, due]) => (
                            <p data-component="TaskRow" {...style.attrs(styles.row)}>
                                <span {...style.attrs(styles.id)}>{id}</span>
                                <span {...style.attrs(styles.rowTitle)}>{title}</span>
                                <span {...style.attrs(styles.quiet)}>{due}</span>
                            </p>
                        ))}
                    </Column>
                    <Column icon="table" name="Signups" count="1,840 · +12 today" type="Signup">
                        {signups.map(([email, joined]) => (
                            <p data-component="SignupRow" {...style.attrs(styles.row)}>
                                <span {...style.attrs(styles.rowTitle)}>{email}</span>
                                <span {...style.attrs(styles.quiet)}>{joined}</span>
                            </p>
                        ))}
                    </Column>
                    <Column icon="calendar" name="Demos booked" count="5" type="Event">
                        {demos.map(([time, person]) => (
                            <p data-component="DemoRow" {...style.attrs(styles.row)}>
                                <span {...style.attrs(styles.rowTitle)}>{person}</span>
                                <span {...style.attrs(styles.quiet)}>{time}</span>
                            </p>
                        ))}
                    </Column>
                </div>
                <p data-component="SpaceAccess" {...style.attrs(styles.access)}>
                    <Glyph size={15} name="person" />
                    Your team and your agent
                </p>
            </div>
        </>
    );
}

/** Draw the tracker's board of your launch issues, its filter finding nothing beyond them. */
function TrackerBoard() {
    return (
        <>
            <p {...style.attrs(styles.bar)}>
                <Favicon icon="linear" tint="#5e6ad2" />
                <span {...style.attrs(styles.title)}>Launch</span>
                <span {...style.attrs(styles.tabs)}>
                    <span {...style.attrs(styles.tab)}>List</span>
                    <span {...style.attrs(styles.tab, styles.tabOn)}>Board</span>
                </span>
                <span {...style.attrs(styles.filterButton, styles.pushed)}>
                    <Glyph size={13} name="filter" />
                    Filter
                </span>
            </p>
            <div data-component="TrackerBoard" {...style.attrs(styles.body, styles.trackerBody)}>
                <div {...style.attrs(styles.lanes)}>
                    {lanes.map(([lane, issues]) => (
                        <section {...style.attrs(styles.lane)}>
                            <p {...style.attrs(styles.laneHead)}>
                                {lane}
                                <span {...style.attrs(styles.quiet)}>{issues.length}</span>
                            </p>
                            {issues.map(([id, title]) => (
                                <p {...style.attrs(styles.card)}>
                                    <span {...style.attrs(styles.id)}>{id}</span>
                                    <span {...style.attrs(styles.cardTitle)}>{title}</span>
                                </p>
                            ))}
                        </section>
                    ))}
                </div>

                {/* the filter menu, asked for signups */}
                <div data-component="FilterMenu" {...style.attrs(styles.menu)}>
                    <p {...style.attrs(styles.menuSearch)}>
                        <Glyph size={13} name="search" />
                        signups
                        <span {...style.attrs(styles.caret)} />
                    </p>
                    <p {...style.attrs(styles.menuEmpty)}>No results</p>
                    <p {...style.attrs(styles.menuHead)}>Filter issues by</p>
                    {filters.map((filter) => (
                        <p {...style.attrs(styles.menuItem)}>{filter}</p>
                    ))}
                </div>
            </div>
        </>
    );
}

/** Draw a launch board asked of your agent, the conversation on the left and the board right. */
export function LaunchBoard(properties: { isOpenAt: Stagger; assistant: Assistant }) {
    return (
        <div {...style.attrs(styles.app)}>
            {/* talk to your agent */}
            <Fade
                isOpen={properties.isOpenAt(0)}
                xstyle={styles.conversation}
                isLarge
                component={properties.isOpenAt(0) ? "SpaceChat" : "Chat"}
            >
                <div {...style.attrs(styles.thread)}>
                    <UserMessage>
                        Two days to launch. Make me a launch board: open tasks, waitlist signups and
                        booked demos, in one place.
                    </UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply work="Worked for" time="1m 12s">
                            <p {...style.attrs(styles.paragraph)}>
                                Added a <b {...style.attrs(styles.strong)}>Board</b> view to your
                                Tasks app. It reads{" "}
                                <code {...style.attrs(styles.code)}>Signup</code> from Waitlist and{" "}
                                <code {...style.attrs(styles.code)}>Event</code> from Calendar, so
                                it stays current. Preview it, then merge.
                            </p>
                        </Reply>
                    ) : (
                        <Reply work={properties.assistant.work} time="2m 40s">
                            <p {...style.attrs(styles.paragraph)}>
                                Linear boards only show Linear issues. Signups live in Replit and
                                demos in Calendly, and Linear can't read either.
                            </p>
                        </Reply>
                    )}
                    <UserMessage>Ship it, and share it with the team.</UserMessage>
                    {properties.isOpenAt(0) ? (
                        <Reply work="Worked for" time="9s">
                            <p {...style.attrs(styles.paragraph)}>
                                Merged. Your team sees the board in Tasks, and I can read it too.
                            </p>
                        </Reply>
                    ) : (
                        <Reply work={properties.assistant.work} time="21s">
                            <p {...style.attrs(styles.paragraph)}>
                                There's only the tasks part to share. Want me to send that?
                            </p>
                        </Reply>
                    )}
                </div>
                <div data-component="Composer" {...style.attrs(styles.composer)}>
                    <span {...style.attrs(styles.quiet)}>
                        {properties.isOpenAt(0) ? "Ask your agent" : properties.assistant.prompt}
                    </span>
                    <span {...style.attrs(styles.controls)}>
                        <span {...style.attrs(styles.iconButton)}>
                            <Glyph size={15} name="plus" />
                        </span>
                        {properties.isOpenAt(0) ? (
                            <span data-component="AgentScope" {...style.attrs(styles.chip)}>
                                <ServiceGlyph icon="tasks" />
                                <ServiceGlyph icon="calendar" />
                                Your space
                                <span title="Live" {...style.attrs(styles.dot)} />
                            </span>
                        ) : (
                            <span data-component="ConnectorScope" {...style.attrs(styles.chip)}>
                                3 connectors
                            </span>
                        )}
                        <span {...style.attrs(styles.iconButton, styles.pushed)}>
                            <Glyph size={15} name="microphone" />
                        </span>
                        <span {...style.attrs(styles.send)}>
                            <Glyph size={15} name="up" />
                        </span>
                    </span>
                </div>
            </Fade>

            {/* show the board as a view of your own app, or the vendor's roadmap you wait on */}
            <Fade
                isOpen={properties.isOpenAt(1)}
                xstyle={styles.pane}
                isLarge
                component={properties.isOpenAt(1) ? "SpaceApp" : "TrackerBoard"}
            >
                {properties.isOpenAt(1) ? <TasksBoard /> : <TrackerBoard />}
            </Fade>
        </div>
    );
}

/** The launch board styles. */
const styles = style.create({
    app: {
        display: "grid",
        fontSize: "0.8125rem",
        gridTemplateColumns: {
            default: "minmax(0, 1fr) minmax(0, 1.75fr)",
            [NARROW]: "minmax(0, 1fr)",
        },
        gridTemplateRows: { default: null, [NARROW]: "20rem minmax(0, 1fr)" },
        height: "100%",
        minHeight: 0,
    },
    conversation: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: 0, [NARROW]: stroke.border },
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: { default: stroke.border, [NARROW]: 0 },
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr) auto",
        minHeight: 0,
        paddingBlockStart: "0.75rem",
        paddingBlockEnd: "1rem",
        paddingInline: "1.25rem",
    },
    thread: {
        alignContent: "end",
        display: "grid",
        gap: "1rem",
        maskImage: "linear-gradient(to bottom, transparent, black 2rem)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
    },
    bubble: {
        backgroundColor: color.muted,
        borderRadius: "1.125rem",
        justifySelf: "end",
        lineHeight: 1.45,
        margin: 0,
        maxWidth: "80%",
        paddingBlock: "0.5rem",
        paddingInline: "0.875rem",
    },
    reply: {
        display: "grid",
        gap: "0.375rem",
    },
    worked: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.mutedForeground,
        display: "flex",
        gap: "0.25rem",
        margin: 0,
        paddingBottom: "0.375rem",
    },
    replyText: {
        display: "grid",
        gap: "0.5rem",
        lineHeight: 1.6,
    },
    paragraph: {
        margin: 0,
    },
    code: {
        backgroundColor: color.muted,
        borderRadius: "4px",
        fontFamily: font.code,
        fontSize: "0.75rem",
        paddingInline: "0.25rem",
    },
    actions: {
        color: color.mutedForeground,
        display: "flex",
        gap: "0.75rem",
        margin: 0,
    },
    composer: {
        borderColor: color.border,
        borderRadius: "1.25rem",
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        display: "grid",
        gap: "0.625rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        marginTop: "2rem",
        paddingBlockStart: "0.75rem",
        paddingBlockEnd: "0.5rem",
        paddingInlineStart: "1rem",
        paddingInlineEnd: "0.5rem",
    },
    controls: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
        marginLeft: "-0.5rem",
        minWidth: 0,
    },
    iconButton: {
        alignItems: "center",
        borderRadius: "50%",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        height: "1.75rem",
        justifyContent: "center",
        width: "1.75rem",
    },
    chip: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "inline-flex",
        fontSize: "0.72rem",
        gap: "0.25rem",
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: "0.1875rem",
        paddingInlineStart: "0.25rem",
        paddingInlineEnd: "0.625rem",
        whiteSpace: "nowrap",
    },
    pushed: {
        marginLeft: "auto",
    },
    send: {
        alignItems: "center",
        backgroundColor: color.foreground,
        borderRadius: "50%",
        color: color.background,
        display: "flex",
        flexShrink: 0,
        height: "1.875rem",
        justifyContent: "center",
        width: "1.875rem",
    },
    pane: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        minWidth: 0,
    },
    bar: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "0.5rem",
        height: "2.75rem",
        paddingInlineStart: "1rem",
        paddingInlineEnd: "1rem",
    },
    title: {
        fontWeight: 600,
        whiteSpace: "nowrap",
    },
    dot: {
        backgroundColor: palette.lightGreen,
        borderRadius: "50%",
        height: "0.5rem",
        width: "0.5rem",
    },
    body: {
        display: "grid",
        gap: "1rem",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: 0,
        overflow: "hidden",
        paddingBlock: { default: "1.25rem", [PHONE]: "1rem" },
        paddingInline: { default: "1.5rem", [PHONE]: "1rem" },
    },
    change: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        minHeight: "2.75rem",
        minWidth: 0,
        paddingInline: "0.875rem",
        flexWrap: { default: "nowrap", [PHONE]: "wrap" },
        paddingBlock: { default: "0", [PHONE]: "0.5rem" },
        whiteSpace: "nowrap",
    },
    added: {
        color: palette.green,
        fontFamily: font.code,
        fontSize: "0.75rem",
        fontWeight: 600,
    },
    button: {
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        color: color.foreground,
        flexShrink: 0,
        fontSize: "0.75rem",
        fontWeight: 600,
        paddingBlock: "0.1875rem",
        paddingInline: "0.625rem",
        whiteSpace: "nowrap",
    },
    tabs: {
        display: "flex",
        gap: "0.25rem",
        marginLeft: "0.75rem",
    },
    tab: {
        alignItems: "center",
        borderRadius: "6px",
        color: color.mutedForeground,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        paddingBlock: "0.25rem",
        paddingInline: "0.5rem",
    },
    tabOn: {
        backgroundColor: color.muted,
        color: color.foreground,
        fontWeight: 600,
    },
    new: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 16%, transparent)`,
        borderRadius: "4px",
        color: palette.ember,
        fontSize: "0.625rem",
        fontWeight: 700,
        paddingInline: "0.25rem",
    },
    trackerBody: {
        gridTemplateRows: "minmax(0, 1fr)",
        position: "relative",
    },
    filterButton: {
        alignItems: "center",
        borderColor: color.border,
        borderRadius: "6px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "inline-flex",
        fontSize: "0.75rem",
        gap: "0.375rem",
        paddingBlock: "0.1875rem",
        paddingInline: "0.5rem",
    },
    lanes: {
        display: "grid",
        gap: "0.75rem",
        gridTemplateColumns: { default: "repeat(3, minmax(0, 1fr))", [PHONE]: "minmax(0, 1fr)" },
        minHeight: 0,
        paddingInlineEnd: { default: "15rem", [PHONE]: 0 },
    },
    lane: {
        alignContent: "start",
        backgroundColor: color.muted,
        borderRadius: "8px",
        display: "grid",
        gap: "0.5rem",
        padding: "0.625rem",
    },
    laneHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "0.75rem",
        fontWeight: 600,
        gap: "0.5rem",
        margin: 0,
    },
    card: {
        backgroundColor: color.card,
        borderRadius: "6px",
        boxShadow: `0 0 0 1px ${color.border}`,
        display: "grid",
        gap: "0.25rem",
        margin: 0,
        paddingBlock: "0.5rem",
        paddingInline: "0.625rem",
    },
    cardTitle: {
        fontSize: "0.8125rem",
        fontWeight: 600,
    },
    menu: {
        backgroundColor: color.card,
        borderRadius: "8px",
        boxShadow: `0 0 0 1px ${color.border}, 0 12px 28px color-mix(in srgb, ${palette.ink} 18%, transparent)`,
        display: "grid",
        fontSize: "0.8125rem",
        paddingBlock: "0.375rem",
        position: "absolute",
        right: "1.5rem",
        top: "0.5rem",
        width: { default: "14rem", [PHONE]: "calc(100% - 2rem)" },
    },
    menuSearch: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        paddingBottom: "0.5rem",
        paddingTop: "0.375rem",
        paddingInline: "0.75rem",
    },
    caret: {
        backgroundColor: color.foreground,
        height: "1em",
        marginLeft: "-0.375rem",
        width: "1.5px",
    },
    menuEmpty: {
        color: color.mutedForeground,
        margin: 0,
        paddingBlock: "0.625rem",
        paddingInline: "0.75rem",
    },
    menuHead: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: stroke.border,
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        margin: 0,
        paddingBottom: "0.25rem",
        paddingTop: "0.5rem",
        paddingInline: "0.75rem",
    },
    menuItem: {
        color: color.mutedForeground,
        margin: 0,
        opacity: 0.7,
        paddingBlock: "0.25rem",
        paddingInline: "0.75rem",
    },
    columns: {
        alignItems: "stretch",
        display: "grid",
        gap: "0.75rem",
        gridAutoColumns: { default: "auto", [PHONE]: "82%" },
        gridAutoFlow: { default: "row", [PHONE]: "column" },
        gridTemplateColumns: { default: "repeat(3, minmax(0, 1fr))", [PHONE]: "none" },
        minHeight: 0,
        overflowX: { default: "visible", [PHONE]: "auto" },
        scrollSnapType: { default: "none", [PHONE]: "x mandatory" },
    },
    panel: {
        scrollSnapAlign: "start",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: stroke.border,
        display: "grid",
        minWidth: 0,
        overflow: "hidden",
    },
    panelHead: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        display: "flex",
        fontSize: "0.75rem",
        fontWeight: 600,
        gap: "0.5rem",
        height: "2.25rem",
        margin: 0,
        paddingInline: "0.75rem",
        whiteSpace: "nowrap",
    },
    count: {
        color: color.mutedForeground,
        fontWeight: 500,
        marginLeft: "auto",
    },
    stack: {
        alignContent: "start",
        display: "grid",
        minWidth: 0,
    },
    source: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderTopColor: color.border,
        borderTopStyle: "dashed",
        borderTopWidth: stroke.border,
        color: color.mutedForeground,
        display: "flex",
        flexWrap: "wrap",
        fontSize: "0.6875rem",
        gap: "0.375rem",
        margin: 0,
        minHeight: "2rem",
        paddingBlock: "0.25rem",
        paddingInline: "0.75rem",
    },
    sourceType: {
        color: color.foreground,
        fontSize: "0.6875rem",
        marginLeft: 0,
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: { default: stroke.border, ":last-child": 0 },
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        minHeight: "2.375rem",
        minWidth: 0,
        paddingInline: "0.75rem",
    },
    id: {
        color: color.mutedForeground,
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: "0.6875rem",
    },
    rowTitle: {
        flexGrow: 1,
        fontWeight: 600,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    access: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "flex",
        gap: "0.5rem",
        margin: 0,
        whiteSpace: "nowrap",
    },
    wide: { display: { default: "inline-flex", [PHONE]: "none" } },
    strong: {
        fontWeight: 600,
    },
    quiet: {
        alignItems: "center",
        color: color.mutedForeground,
        display: "inline-flex",
        flexShrink: 0,
        gap: "0.25rem",
    },
});
