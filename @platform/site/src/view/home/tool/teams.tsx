import * as style from "@destack/style";

import { messages } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { teams } from "./teams.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    bell: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M10 20.5a2 2 0 0 0 4 0"],
    chat: ["M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H9l-5 4z"],
    people: [
        "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
        "M3 20a6 6 0 0 1 12 0",
        "M16 11a3 3 0 1 0 0-6",
        "M17.5 14.5A5 5 0 0 1 21 19.5",
    ],
    calendar: ["M4 6h16v14H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
    phone: [
        "M6 3h3l2 5-2.5 1.5a11 11 0 0 0 6 6L16 13l5 2v3a2 2 0 0 1-2 2A16 16 0 0 1 4 5a2 2 0 0 1 2-2z",
    ],
    cloud: ["M7 18a4 4 0 0 1-.5-8A6 6 0 0 1 18 9a4.5 4.5 0 0 1-.5 9z"],
    apps: ["M5 5h5v5H5z", "M14 5h5v5h-5z", "M5 14h5v5H5z", "M14 14h5v5h-5z"],
    back: ["m15 18-6-6 6-6"],
    forward: ["m9 18 6-6-6-6"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    more: ["M5 12h.01", "M12 12h.01", "M19 12h.01"],
    filter: ["M4 6h16", "M7 12h10", "M10 18h4"],
    plus: ["M12 5v14", "M5 12h14"],
    video: ["m16 10 5-3v10l-5-3", "M3 6h13v12H3z"],
    caret: ["m7 10 5 5 5-5"],
    info: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 11v6", "M12 7.5h.01"],
    compose: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    reply: ["M10 8 5 12l5 4", "M5 12h9a5 5 0 0 1 5 5v1"],
} satisfies Record<string, readonly string[]>;

/** The app bar's places: name and glyph, Teams chosen. */
const places: readonly (readonly [name: string, glyph: keyof typeof glyphs])[] = [
    ["Activity", "bell"],
    ["Chat", "chat"],
    ["Teams", "people"],
    ["Calendar", "calendar"],
    ["Calls", "phone"],
    ["OneDrive", "cloud"],
    ["Apps", "apps"],
];

/** The team's channels, launch chosen, with whether each has unread posts. */
const channels: readonly (readonly [name: string, isUnread: boolean])[] = [
    ["General", false],
    ["launch", false],
    ["design", true],
    ["pricing", false],
];

/** Draw the launch channel in the office chat app, each message a post with its replies. */
export function Teams() {
    return (
        <div data-component="TeamsPage" {...style.attrs(styles.page)}>
            {/* title bar */}
            <header {...style.attrs(styles.title)}>
                <Mark name="back" size={16} />
                <Mark name="forward" size={16} />
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={14} />
                    Search (⌘E)
                </span>
                <span {...style.attrs(styles.titleEnd)}>
                    <Mark name="more" size={16} />
                    <span {...style.attrs(styles.me)}>
                        F
                        <span {...style.attrs(styles.available)} />
                    </span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* app bar */}
                <nav {...style.attrs(styles.bar)}>
                    {places.map(([name, glyph]) => (
                        <span {...style.attrs(styles.place, name === "Teams" && styles.placeOn)}>
                            <span
                                {...style.attrs(
                                    styles.placeIcon,
                                    name === "Teams" && styles.placeIconOn,
                                )}
                            >
                                <Mark name={glyph} size={20} />
                            </span>
                            {name}
                        </span>
                    ))}
                </nav>

                {/* teams and channels */}
                <nav {...style.attrs(styles.pane)}>
                    <span {...style.attrs(styles.paneHead)}>
                        <b>Teams</b>
                        <span {...style.attrs(styles.paneTools)}>
                            <Mark name="filter" size={16} />
                            <Mark name="plus" size={16} />
                        </span>
                    </span>
                    <span {...style.attrs(styles.team)}>
                        <Mark name="caret" size={12} />
                        <span {...style.attrs(styles.teamMark)}>S</span>
                        Symbol
                    </span>
                    {channels.map(([name, isUnread]) => (
                        <span
                            {...style.attrs(
                                styles.channel,
                                isUnread && styles.channelUnread,
                                name === "launch" && styles.channelOn,
                            )}
                        >
                            {name}
                        </span>
                    ))}
                </nav>

                {/* the channel */}
                <main {...style.attrs(styles.main)}>
                    <header {...style.attrs(styles.head)}>
                        <span {...style.attrs(styles.teamMark, styles.teamMarkLarge)}>S</span>
                        <b {...style.attrs(styles.channelName)}>launch</b>
                        <span {...style.attrs(styles.tab, styles.tabOn)}>Posts</span>
                        <span {...style.attrs(styles.tab)}>Files</span>
                        <span {...style.attrs(styles.tab, styles.wide)}>Notes</span>
                        <Mark name="plus" size={14} />
                        <span {...style.attrs(styles.headEnd)}>
                            <span {...style.attrs(styles.meet)}>
                                <Mark name="video" size={15} />
                                Meet
                                <span {...style.attrs(styles.meetCaret)}>
                                    <Mark name="caret" size={12} />
                                </span>
                            </span>
                            <Mark name="info" size={18} />
                        </span>
                    </header>

                    <ol {...style.attrs(styles.posts)}>
                        {messages.map(([author, time, text], index) => (
                            <li {...style.attrs(styles.post)}>
                                <span {...style.attrs(styles.postBody)}>
                                    <span
                                        {...style.attrs(
                                            styles.face,
                                            author === "Agent"
                                                ? styles.faceAgent
                                                : styles.faceFlorian,
                                        )}
                                    >
                                        {author === "Agent" ? "A" : "F"}
                                    </span>
                                    <span {...style.attrs(styles.postText)}>
                                        <span {...style.attrs(styles.byline)}>
                                            <b>{author === "Agent" ? "Agent" : "Florian Cäsar"}</b>
                                            {author === "Agent" ? (
                                                <span {...style.attrs(styles.bot)}>Bot</span>
                                            ) : undefined}
                                            <span {...style.attrs(styles.time)}>{time}</span>
                                        </span>
                                        {text}
                                        {index === 1 ? (
                                            <span {...style.attrs(styles.reactions)}>
                                                <span {...style.attrs(styles.reaction)}>👍 2</span>
                                            </span>
                                        ) : undefined}
                                    </span>
                                </span>
                                {index === 2 ? (
                                    <span {...style.attrs(styles.replies)}>
                                        <span {...style.attrs(styles.replyFace)}>F</span>3 replies
                                        from Florian
                                    </span>
                                ) : undefined}
                                <span {...style.attrs(styles.replyBar)}>
                                    <Mark name="reply" size={14} />
                                    Reply
                                </span>
                            </li>
                        ))}
                    </ol>

                    <span {...style.attrs(styles.compose)}>
                        <span {...style.attrs(styles.postButton)}>
                            <Mark name="compose" size={15} />
                            Post in channel
                        </span>
                    </span>
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

/** The Teams page styles. */
const styles = style.create({
    page: {
        backgroundColor: teams.frame,
        color: teams.ink,
        display: "grid",
        fontFamily: teams.face,
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
        strokeWidth: 1.6,
    },

    // title bar
    title: {
        alignItems: "center",
        color: teams.muted,
        display: "flex",
        gap: "12px",
        height: "48px",
        paddingInline: "16px",
    },
    search: {
        alignItems: "center",
        backgroundColor: teams.surface,
        borderRadius: "4px",
        display: { default: "flex", [site.narrow]: "none" },
        gap: "8px",
        marginInline: "auto",
        maxWidth: "460px",
        paddingBlock: "6px",
        paddingInline: "10px",
        width: "50%",
    },
    titleEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: { default: "0", [site.narrow]: "auto" },
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: teams.onAccent,
        display: "inline-flex",
        fontSize: "12px",
        height: "28px",
        justifyContent: "center",
        position: "relative",
        width: "28px",
    },
    available: {
        backgroundColor: teams.available,
        borderColor: teams.frame,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "2px",
        bottom: "-2px",
        height: "8px",
        insetInlineEnd: "-2px",
        position: "absolute",
        width: "8px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: {
            default: "68px 240px minmax(0, 1fr)",
            [site.narrow]: "minmax(0, 1fr)",
        },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // app bar
    bar: {
        alignItems: "center",
        color: teams.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "6px",
        paddingBlockStart: "4px",
    },
    place: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
        fontSize: "10px",
        gap: "2px",
        paddingBlock: "4px",
        position: "relative",
        width: "100%",
    },
    placeOn: {
        color: teams.brand,
        fontWeight: 600,
        "::before": {
            backgroundColor: teams.brand,
            borderRadius: "2px",
            content: '""',
            height: "24px",
            insetInlineStart: "4px",
            position: "absolute",
            top: "8px",
            width: "3px",
        },
    },
    placeIcon: {
        alignItems: "center",
        borderRadius: "6px",
        display: "inline-flex",
        height: "32px",
        justifyContent: "center",
        width: "40px",
    },
    placeIconOn: {
        backgroundColor: teams.tint,
    },

    // teams pane
    pane: {
        backgroundColor: teams.pane,
        borderStartStartRadius: "8px",
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "2px",
        paddingInline: "8px",
    },
    paneHead: {
        alignItems: "center",
        display: "flex",
        fontSize: "18px",
        height: "52px",
        justifyContent: "space-between",
        paddingInline: "8px",
    },
    paneTools: {
        color: teams.muted,
        display: "flex",
        gap: "12px",
    },
    team: {
        alignItems: "center",
        display: "flex",
        fontWeight: 600,
        gap: "8px",
        paddingBlock: "6px",
        paddingInline: "4px",
    },
    teamMark: {
        alignItems: "center",
        backgroundColor: site.space,
        borderRadius: "4px",
        color: teams.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "11px",
        fontWeight: 600,
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    teamMarkLarge: {
        borderRadius: "6px",
        fontSize: "13px",
        height: "32px",
        width: "32px",
    },
    channel: {
        borderRadius: "4px",
        color: teams.muted,
        paddingBlock: "6px",
        paddingInlineEnd: "8px",
        paddingInlineStart: "48px",
    },
    channelUnread: {
        color: teams.ink,
        fontWeight: 600,
    },
    channelOn: {
        backgroundColor: teams.surface,
        color: teams.ink,
        fontWeight: 600,
    },

    // the channel
    main: {
        backgroundColor: teams.canvas,
        borderInlineStartColor: teams.line,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        backgroundColor: teams.surface,
        borderBottomColor: teams.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: teams.muted,
        display: "flex",
        gap: "14px",
        height: "56px",
        paddingInline: "16px",
    },
    channelName: {
        color: teams.ink,
        fontSize: "16px",
    },
    tab: {
        alignSelf: "stretch",
        alignItems: "center",
        display: "flex",
    },
    tabOn: {
        boxShadow: `inset 0 -3px 0 ${teams.brand}`,
        color: teams.ink,
        fontWeight: 600,
    },
    wide: {
        display: { default: "flex", [site.narrow]: "none" },
    },
    headEnd: {
        alignItems: "center",
        display: "flex",
        gap: "14px",
        marginInlineStart: "auto",
    },
    meet: {
        alignItems: "center",
        backgroundColor: teams.brand,
        borderRadius: "4px",
        color: teams.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "6px",
        height: "32px",
        paddingInlineStart: "12px",
    },
    meetCaret: {
        alignItems: "center",
        borderInlineStartColor: teams.split,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        display: "inline-flex",
        height: "100%",
        paddingInline: "6px",
    },
    posts: {
        display: "flex",
        flexDirection: "column",
        gap: "12px",
        justifyContent: "flex-end",
        listStyle: "none",
        margin: 0,
        maskImage: "linear-gradient(to bottom, transparent, black 3rem)",
        overflow: "hidden",
        paddingBlock: "16px",
        paddingInline: "20px",
    },
    post: {
        backgroundColor: teams.surface,
        borderRadius: "6px",
        boxShadow: teams.lift,
        display: "grid",
        flexShrink: 0,
    },
    postBody: {
        display: "flex",
        gap: "12px",
        paddingBlock: "12px",
        paddingInline: "16px",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: teams.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "12px",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    faceAgent: {
        backgroundColor: site.agent,
    },
    faceFlorian: {
        backgroundColor: site.avatar,
    },
    postText: {
        display: "grid",
        gap: "4px",
        lineHeight: 1.43,
    },
    byline: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    bot: {
        backgroundColor: teams.tint,
        borderRadius: "4px",
        color: teams.brand,
        fontSize: "11px",
        fontWeight: 600,
        paddingInline: "4px",
    },
    time: {
        color: teams.muted,
        fontSize: "12px",
    },
    reactions: {
        display: "flex",
    },
    reaction: {
        backgroundColor: teams.tint,
        borderRadius: "999px",
        fontSize: "12px",
        paddingBlock: "1px",
        paddingInline: "8px",
    },
    replies: {
        alignItems: "center",
        borderTopColor: teams.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        color: teams.brand,
        display: "flex",
        fontSize: "12px",
        fontWeight: 600,
        gap: "8px",
        paddingBlock: "6px",
        paddingInline: "60px",
    },
    replyFace: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: teams.onAccent,
        display: "inline-flex",
        fontSize: "9px",
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    replyBar: {
        alignItems: "center",
        borderTopColor: teams.line,
        borderTopStyle: "solid",
        borderTopWidth: "1px",
        color: teams.muted,
        display: "flex",
        fontSize: "12px",
        gap: "6px",
        paddingBlock: "8px",
        paddingInline: "60px",
    },
    compose: {
        display: "flex",
        paddingBlockEnd: "16px",
        paddingInline: "20px",
    },
    postButton: {
        alignItems: "center",
        backgroundColor: teams.brand,
        borderRadius: "4px",
        color: teams.onAccent,
        display: "inline-flex",
        fontWeight: 600,
        gap: "8px",
        height: "32px",
        paddingInline: "12px",
    },
});
