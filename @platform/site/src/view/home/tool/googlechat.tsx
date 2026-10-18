import * as style from "@destack/style";

import { messages } from "../verb/corner";
import { Favicon } from "../figure/glyph";
import { site } from "../figure/site.stylex";
import { googleChat } from "./googlechat.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    menu: ["M4 7h16", "M4 12h16", "M4 17h16"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
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
    apps: [
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
    pencil: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    home: ["M4 11 12 4l8 7v9h-5v-6H9v6H4z"],
    at: [
        "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
        "M16 12v1.5a2.5 2.5 0 0 0 5 0V12a9 9 0 1 0-3.5 7.1",
    ],
    star: ["m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"],
    video: ["m16 10 5-3v10l-5-3", "M3 6h13v12H3z"],
    side: ["M4 5h16v14H4z", "M15 5v14"],
    caret: ["m7 10 5 5 5-5"],
    plus: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 8v8", "M8 12h8"],
    format: ["M6 18 11 6h2l5 12", "M8 14h8"],
    smile: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M8.5 14.5a4.5 4.5 0 0 0 7 0",
        "M9 10h.01",
        "M15 10h.01",
    ],
    upload: ["M12 16V5", "m7 10 5-5 5 5", "M5 19h14"],
    send: ["M4 12 20 4l-4 16-4-7z", "m12 13 8-9"],
} satisfies Record<string, readonly string[]>;

/** The people the sidebar lists under direct messages: name and colour. */
const people: readonly (readonly [name: string, tint: string])[] = [
    ["Maya Chen", "#a0485f"],
    ["Jonas Weber", "#2f7d8c"],
];

/** The spaces the sidebar lists, the launch space chosen: name and emoji. */
const spaces: readonly (readonly [name: string, emoji: string])[] = [
    ["Launch", "🚀"],
    ["Design", "🎨"],
    ["General", "💬"],
];

/** Draw the launch conversation as a space in the workspace's chat. */
export function GoogleChat() {
    return (
        <div data-component="GoogleChatPage" {...style.attrs(styles.page)}>
            {/* top bar */}
            <header {...style.attrs(styles.top)}>
                <Mark name="menu" size={22} />
                <span {...style.attrs(styles.brand)}>
                    <Favicon icon="googlechat" tint="#34a853" size={28} />
                    Chat
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={18} />
                    Search in chat
                </span>
                <span {...style.attrs(styles.topEnd)}>
                    <span {...style.attrs(styles.status)}>
                        <span {...style.attrs(styles.statusDot)} />
                        Active
                        <Mark name="caret" size={14} />
                    </span>
                    <Mark name="help" size={20} />
                    <Mark name="gear" size={20} />
                    <Mark name="apps" size={20} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </header>

            <div {...style.attrs(styles.body)}>
                {/* sidebar */}
                <nav {...style.attrs(styles.side)}>
                    <span {...style.attrs(styles.newChat)}>
                        <Mark name="pencil" size={20} />
                        New chat
                    </span>
                    <span {...style.attrs(styles.item)}>
                        <Mark name="home" size={18} />
                        Home
                    </span>
                    <span {...style.attrs(styles.item)}>
                        <Mark name="at" size={18} />
                        Mentions
                    </span>
                    <span {...style.attrs(styles.item)}>
                        <Mark name="star" size={18} />
                        Starred
                    </span>
                    <span {...style.attrs(styles.group)}>Direct messages</span>
                    {people.map(([name, tint]) => (
                        <span {...style.attrs(styles.item)}>
                            <span
                                style={{ "background-color": tint }}
                                {...style.attrs(styles.personFace)}
                            >
                                {name.slice(0, 1)}
                            </span>
                            {name}
                        </span>
                    ))}
                    <span {...style.attrs(styles.group)}>Spaces</span>
                    {spaces.map(([name, emoji]) => (
                        <span {...style.attrs(styles.item, name === "Launch" && styles.itemOn)}>
                            <span {...style.attrs(styles.spaceMark)}>{emoji}</span>
                            {name}
                        </span>
                    ))}
                </nav>

                {/* the space */}
                <main {...style.attrs(styles.main)}>
                    <header {...style.attrs(styles.head)}>
                        <span {...style.attrs(styles.spaceMark, styles.spaceMarkLarge)}>🚀</span>
                        <span {...style.attrs(styles.headText)}>
                            <b>Launch</b>
                            <span {...style.attrs(styles.members)}>
                                4 members
                                <Mark name="caret" size={12} />
                            </span>
                        </span>
                        <span {...style.attrs(styles.headTools)}>
                            <Mark name="search" size={20} />
                            <Mark name="video" size={20} />
                            <Mark name="side" size={20} />
                        </span>
                    </header>
                    <span {...style.attrs(styles.tabs)}>
                        <span {...style.attrs(styles.tabOn)}>Chat</span>
                        <span>Shared</span>
                        <span>Tasks</span>
                    </span>

                    <ol {...style.attrs(styles.messages)}>
                        <li {...style.attrs(styles.day)}>
                            <span {...style.attrs(styles.dayChip)}>Today</span>
                        </li>
                        {messages.map(([author, time, text], index) => (
                            <li {...style.attrs(styles.message)}>
                                <span
                                    {...style.attrs(
                                        styles.face,
                                        author === "Agent" ? styles.faceAgent : styles.faceFlorian,
                                    )}
                                >
                                    {author === "Agent" ? "A" : "F"}
                                </span>
                                <span {...style.attrs(styles.messageBody)}>
                                    <span {...style.attrs(styles.byline)}>
                                        <b>{author === "Agent" ? "Agent" : "Florian Cäsar"}</b>
                                        {author === "Agent" ? (
                                            <span {...style.attrs(styles.app)}>App</span>
                                        ) : undefined}
                                        <span {...style.attrs(styles.time)}>{time}</span>
                                    </span>
                                    <span>{text}</span>
                                    {index === 2 ? (
                                        <span {...style.attrs(styles.thread)}>
                                            <span {...style.attrs(styles.threadFace)}>F</span>3
                                            replies
                                            <span {...style.attrs(styles.time)}>
                                                Last reply 09:52
                                            </span>
                                        </span>
                                    ) : undefined}
                                </span>
                            </li>
                        ))}
                    </ol>

                    <span {...style.attrs(styles.composer)}>
                        <Mark name="plus" size={22} />
                        <span {...style.attrs(styles.placeholder)}>History is on</span>
                        <span {...style.attrs(styles.composerTools)}>
                            <Mark name="format" size={18} />
                            <Mark name="smile" size={18} />
                            <Mark name="upload" size={18} />
                            <Mark name="video" size={18} />
                        </span>
                        <Mark name="send" size={18} />
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

/** The Google Chat page styles. */
const styles = style.create({
    page: {
        backgroundColor: googleChat.frame,
        color: googleChat.ink,
        display: "grid",
        fontFamily: googleChat.face,
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
        color: googleChat.muted,
        display: "flex",
        gap: "16px",
        height: "64px",
        paddingInline: "16px",
    },
    brand: {
        alignItems: "center",
        color: googleChat.muted,
        display: "flex",
        fontFamily: googleChat.display,
        fontSize: "22px",
        gap: "8px",
        minWidth: { default: "184px", [site.narrow]: "0" },
    },
    search: {
        alignItems: "center",
        backgroundColor: googleChat.field,
        borderRadius: "28px",
        display: { default: "flex", [site.narrow]: "none" },
        flexGrow: 1,
        fontSize: "16px",
        gap: "14px",
        height: "48px",
        maxWidth: "720px",
        paddingInline: "16px",
    },
    topEnd: {
        alignItems: "center",
        display: "flex",
        gap: "16px",
        marginInlineStart: "auto",
    },
    status: {
        alignItems: "center",
        borderColor: googleChat.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleChat.ink,
        display: { default: "flex", [site.narrow]: "none" },
        gap: "6px",
        height: "32px",
        paddingInline: "12px",
    },
    statusDot: {
        backgroundColor: googleChat.active,
        borderRadius: "50%",
        height: "8px",
        width: "8px",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: googleChat.onAccent,
        display: "inline-flex",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },
    body: {
        display: "grid",
        gridTemplateColumns: { default: "256px minmax(0, 1fr)", [site.narrow]: "minmax(0, 1fr)" },
        gridTemplateRows: "minmax(0, 1fr)",
        minHeight: 0,
    },

    // sidebar
    side: {
        color: googleChat.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "2px",
        paddingInline: "8px",
    },
    newChat: {
        alignItems: "center",
        backgroundColor: googleChat.button,
        borderRadius: "16px",
        color: googleChat.ink,
        display: "inline-flex",
        fontFamily: googleChat.display,
        fontWeight: 500,
        gap: "12px",
        height: "56px",
        marginBlockEnd: "12px",
        paddingInlineEnd: "20px",
        paddingInlineStart: "16px",
        width: "fit-content",
    },
    item: {
        alignItems: "center",
        borderRadius: "999px",
        display: "flex",
        gap: "16px",
        height: "32px",
        paddingInline: "16px",
    },
    itemOn: {
        backgroundColor: googleChat.selected,
        color: googleChat.ink,
        fontWeight: 600,
    },
    group: {
        color: googleChat.ink,
        fontFamily: googleChat.display,
        fontWeight: 500,
        paddingBlockEnd: "4px",
        paddingBlockStart: "16px",
        paddingInline: "16px",
    },
    personFace: {
        alignItems: "center",
        borderRadius: "50%",
        color: googleChat.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "11px",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    spaceMark: {
        alignItems: "center",
        backgroundColor: googleChat.field,
        borderRadius: "6px",
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "12px",
        height: "20px",
        justifyContent: "center",
        width: "20px",
    },
    spaceMarkLarge: {
        borderRadius: "10px",
        fontSize: "18px",
        height: "36px",
        width: "36px",
    },

    // the space
    main: {
        backgroundColor: googleChat.surface,
        borderRadius: { default: "16px", [site.narrow]: "0" },
        display: "grid",
        gridTemplateRows: "auto auto minmax(0, 1fr) auto",
        marginBlockEnd: { default: "16px", [site.narrow]: "0" },
        marginInlineEnd: { default: "16px", [site.narrow]: "0" },
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        paddingBlockStart: "12px",
        paddingInline: "16px",
    },
    headText: {
        display: "grid",
        flexGrow: 1,
        fontFamily: googleChat.display,
        fontSize: "18px",
    },
    members: {
        alignItems: "center",
        color: googleChat.muted,
        display: "flex",
        fontFamily: googleChat.face,
        fontSize: "12px",
        gap: "2px",
    },
    headTools: {
        color: googleChat.muted,
        display: "flex",
        gap: "20px",
    },
    tabs: {
        borderBottomColor: googleChat.field,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: googleChat.muted,
        display: "flex",
        fontFamily: googleChat.display,
        fontWeight: 500,
        gap: "24px",
        paddingBlock: "10px",
        paddingInline: "64px",
    },
    tabOn: {
        boxShadow: `0 13px 0 -10px ${googleChat.link}`,
        color: googleChat.link,
    },
    messages: {
        display: "flex",
        flexDirection: "column",
        gap: "18px",
        justifyContent: "flex-end",
        listStyle: "none",
        margin: 0,
        maskImage: "linear-gradient(to bottom, transparent, black 2rem)",
        overflow: "hidden",
        paddingBlock: "12px",
        paddingInline: "20px",
    },
    day: {
        display: "flex",
        justifyContent: "center",
    },
    dayChip: {
        borderColor: googleChat.line,
        borderRadius: "999px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleChat.muted,
        fontSize: "12px",
        fontWeight: 500,
        paddingBlock: "2px",
        paddingInline: "12px",
    },
    message: {
        display: "flex",
        flexShrink: 0,
        gap: "12px",
    },
    face: {
        alignItems: "center",
        borderRadius: "50%",
        color: googleChat.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "13px",
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
    messageBody: {
        display: "grid",
        gap: "2px",
        lineHeight: 1.43,
    },
    byline: {
        alignItems: "center",
        display: "flex",
        gap: "8px",
    },
    app: {
        borderColor: googleChat.line,
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: googleChat.muted,
        fontSize: "11px",
        paddingInline: "4px",
    },
    time: {
        color: googleChat.muted,
        fontSize: "12px",
        fontWeight: 400,
    },
    thread: {
        alignItems: "center",
        color: googleChat.link,
        display: "flex",
        fontSize: "13px",
        fontWeight: 500,
        gap: "8px",
        marginBlockStart: "4px",
    },
    threadFace: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: googleChat.onAccent,
        display: "inline-flex",
        fontSize: "9px",
        height: "18px",
        justifyContent: "center",
        width: "18px",
    },
    composer: {
        alignItems: "center",
        backgroundColor: googleChat.field,
        borderRadius: "24px",
        color: googleChat.muted,
        display: "flex",
        gap: "12px",
        marginBlockEnd: "16px",
        marginInline: "16px",
        minHeight: "48px",
        paddingInlineEnd: "16px",
        paddingInlineStart: "12px",
    },
    placeholder: {
        flexGrow: 1,
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    composerTools: {
        display: { default: "flex", [site.narrow]: "none" },
        gap: "14px",
    },
});
