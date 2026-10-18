import * as style from "@destack/style";

import { messages } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { discord } from "./discord.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    hash: ["M10 3 8 21", "M16 3l-2 18", "M4 8.5h16", "M3.5 15.5h16"],
    speaker: ["M4 9.5h3l5-4.5v14l-5-4.5H4z", "M15.5 8.5a5 5 0 0 1 0 7", "M18 6a8.5 8.5 0 0 1 0 12"],
    threads: ["M5 5h14v10H9l-4 4z", "M9 9h6", "M9 12h4"],
    bell: ["M6 16V11a6 6 0 0 1 12 0v5l2 2H4z", "M10 20a2 2 0 0 0 4 0"],
    pin: ["M9 3h6l-1 6 4 4H6l4-4z", "M12 13v8"],
    members: [
        "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
        "M2.5 20a6.5 6.5 0 0 1 13 0",
        "M16 4.5a3.5 3.5 0 0 1 0 6.5",
        "M18 14a6 6 0 0 1 3.5 6",
    ],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    inbox: ["M4 13h4.5l1.5 3h4l1.5-3H20", "M6 5h12l2 8v6H4v-6z"],
    help: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6",
        "M12 17h.01",
    ],
    mic: [
        "M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3z",
        "M19 11a7 7 0 0 1-14 0",
        "M12 18v3",
    ],
    headphones: [
        "M4 15v-3a8 8 0 0 1 16 0v3",
        "M4 15h3v5H5a1 1 0 0 1-1-1z",
        "M20 15h-3v5h2a1 1 0 0 0 1-1z",
    ],
    gear: [
        "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
    ],
    plus: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 8v8", "M8 12h8"],
    gift: [
        "M4 10h16v4H4z",
        "M6 14h12v7H6z",
        "M12 10v11",
        "M12 10C10 6 7 6 7 8s5 2 5 2 5 0 5-2-3-2-5 2",
    ],
    sticker: ["M5 4h14v9l-6 7H5z", "M13 20v-5a2 2 0 0 1 2-2h4"],
    smile: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M8.5 14.5a4.5 4.5 0 0 0 7 0",
        "M9 10h.01",
        "M15 10h.01",
    ],
    caret: ["m7 10 5 5 5-5"],
    compass: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "m15.5 8.5-2 5-5 2 2-5z"],
    check: ["m6 12.5 4 4 8-9"],
} satisfies Record<string, readonly string[]>;

/** The server's channels by category: name and whether it holds unread messages. */
const categories: readonly (readonly [
    name: string,
    channels: readonly (readonly [string, boolean])[],
])[] = [
    ["Information", [["announcements", false]]],
    [
        "Text channels",
        [
            ["general", true],
            ["launch", false],
            ["design", false],
            ["random", true],
        ],
    ],
];

/** The servers on the rail after Symbol: their initials and colours. */
const servers: readonly (readonly [initial: string, tint: string])[] = [
    ["Bn", "#3ba55c"],
    ["TS", "#3178c6"],
];

/** Draw the launch channel on the team's server, in the app's default dark theme. */
export function Discord() {
    return (
        <div data-component="DiscordPage" {...style.attrs(styles.page)}>
            {/* server rail */}
            <nav {...style.attrs(styles.rail)}>
                <span {...style.attrs(styles.server, styles.home)}>
                    <svg viewBox="0 0 24 24" {...style.attrs(styles.logo)}>
                        <path d="M19.3 5.3A16.6 16.6 0 0 0 15.2 4l-.5 1a15.4 15.4 0 0 0-5.4 0L8.8 4a16.5 16.5 0 0 0-4.1 1.3A17.4 17.4 0 0 0 1.8 17a16.7 16.7 0 0 0 5 2.5l1.1-1.7a10.7 10.7 0 0 1-1.7-.8l.4-.3a11.9 11.9 0 0 0 10.8 0l.4.3a10.8 10.8 0 0 1-1.7.8l1.1 1.7a16.6 16.6 0 0 0 5-2.5 17.3 17.3 0 0 0-2.9-11.7zM8.7 14.6c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2zm6.6 0c-1 0-1.8-.9-1.8-2s.8-2 1.8-2 1.8.9 1.8 2-.8 2-1.8 2z" />
                    </svg>
                </span>
                <span {...style.attrs(styles.separator)} />
                <span {...style.attrs(styles.slot)}>
                    <span {...style.attrs(styles.pill)} />
                    <span {...style.attrs(styles.server, styles.chosen)}>S</span>
                </span>
                {servers.map(([initial, tint]) => (
                    <span {...style.attrs(styles.slot)}>
                        <span {...style.attrs(styles.unread)} />
                        <span style={{ "background-color": tint }} {...style.attrs(styles.server)}>
                            {initial}
                        </span>
                    </span>
                ))}
                <span {...style.attrs(styles.server, styles.add)}>
                    <Mark name="plus" size={22} />
                </span>
                <span {...style.attrs(styles.server, styles.add)}>
                    <Mark name="compass" size={22} />
                </span>
            </nav>

            {/* channels */}
            <nav {...style.attrs(styles.sidebar)}>
                <b {...style.attrs(styles.serverName)}>
                    Symbol
                    <Mark name="caret" size={18} />
                </b>
                <div {...style.attrs(styles.channels)}>
                    {categories.map(([category, channels]) => (
                        <>
                            <span {...style.attrs(styles.category)}>
                                <Mark name="caret" size={12} />
                                {category}
                            </span>
                            {channels.map(([name, isUnread]) => (
                                <span
                                    {...style.attrs(
                                        styles.channel,
                                        isUnread && styles.channelUnread,
                                        name === "launch" && styles.channelOn,
                                    )}
                                >
                                    {isUnread ? (
                                        <span {...style.attrs(styles.unreadMark)} />
                                    ) : undefined}
                                    <Mark name="hash" size={20} />
                                    {name}
                                </span>
                            ))}
                        </>
                    ))}
                    <span {...style.attrs(styles.category)}>
                        <Mark name="caret" size={12} />
                        Voice channels
                    </span>
                    <span {...style.attrs(styles.channel)}>
                        <Mark name="speaker" size={20} />
                        Standup
                    </span>
                    <span {...style.attrs(styles.voice)}>
                        <span {...style.attrs(styles.voiceFace, styles.faceFlorian)}>F</span>
                        florian
                    </span>
                    <span {...style.attrs(styles.voice)}>
                        <span {...style.attrs(styles.voiceFace, styles.faceMaya)}>M</span>
                        maya
                    </span>
                </div>

                {/* user panel */}
                <span {...style.attrs(styles.user)}>
                    <span {...style.attrs(styles.userFace)}>
                        F
                        <span {...style.attrs(styles.status)} />
                    </span>
                    <span {...style.attrs(styles.userName)}>
                        <b>florian</b>
                        <span>Online</span>
                    </span>
                    <Mark name="mic" size={18} />
                    <Mark name="headphones" size={18} />
                    <Mark name="gear" size={18} />
                </span>
            </nav>

            {/* the channel */}
            <main {...style.attrs(styles.main)}>
                <header {...style.attrs(styles.head)}>
                    <Mark name="hash" size={22} />
                    <b {...style.attrs(styles.headName)}>launch</b>
                    <span {...style.attrs(styles.topic)}>Ship v4 on Friday</span>
                    <span {...style.attrs(styles.headTools)}>
                        <Mark name="threads" size={20} />
                        <Mark name="bell" size={20} />
                        <Mark name="pin" size={20} />
                        <Mark name="members" size={20} />
                        <span {...style.attrs(styles.search)}>
                            Search
                            <Mark name="search" size={14} />
                        </span>
                        <Mark name="inbox" size={20} />
                    </span>
                </header>

                <ol {...style.attrs(styles.messages)}>
                    <li {...style.attrs(styles.divider)}>
                        <span>October 21, 2025</span>
                    </li>
                    {messages.map(([author, time, text], index) => (
                        <li {...style.attrs(styles.message)}>
                            <span
                                {...style.attrs(
                                    styles.avatar,
                                    author === "Agent" ? styles.faceAgent : styles.faceFlorian,
                                )}
                            >
                                {author === "Agent" ? "A" : "F"}
                            </span>
                            <span {...style.attrs(styles.byline)}>
                                <b
                                    {...style.attrs(
                                        author === "Agent" ? styles.nameAgent : styles.nameOwner,
                                    )}
                                >
                                    {author === "Agent" ? "Agent" : "florian"}
                                </b>
                                {author === "Agent" ? (
                                    <span {...style.attrs(styles.app)}>
                                        <Mark name="check" size={10} />
                                        APP
                                    </span>
                                ) : undefined}
                                <span {...style.attrs(styles.time)}>Today at {time}</span>
                            </span>
                            <span {...style.attrs(styles.text)}>{text}</span>
                            {index === 1 ? (
                                <span {...style.attrs(styles.reactions)}>
                                    <span {...style.attrs(styles.reaction, styles.reactionMine)}>
                                        👍 <b>2</b>
                                    </span>
                                    <span {...style.attrs(styles.reaction)}>
                                        🚀 <b>1</b>
                                    </span>
                                </span>
                            ) : undefined}
                            {index === 2 ? (
                                <span {...style.attrs(styles.thread)}>
                                    <b {...style.attrs(styles.threadName)}>HN titles</b>
                                    <span {...style.attrs(styles.threadCount)}>3 Messages ›</span>
                                    <span {...style.attrs(styles.threadLast)}>
                                        florian: the first one, obviously
                                    </span>
                                </span>
                            ) : undefined}
                        </li>
                    ))}
                </ol>

                {/* composer */}
                <span {...style.attrs(styles.composer)}>
                    <Mark name="plus" size={22} />
                    <span {...style.attrs(styles.placeholder)}>Message #launch</span>
                    <Mark name="gift" size={22} />
                    <span {...style.attrs(styles.gif)}>GIF</span>
                    <Mark name="sticker" size={22} />
                    <Mark name="smile" size={22} />
                </span>
            </main>
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

/** The Discord page styles. */
const styles = style.create({
    page: {
        backgroundColor: discord.rail,
        color: discord.text,
        display: "grid",
        fontFamily: discord.face,
        fontSize: "16px",
        gridTemplateColumns: {
            default: "72px 240px minmax(0, 1fr)",
            [site.narrow]: "minmax(0, 1fr)",
        },
        gridTemplateRows: "minmax(0, 1fr)",
        height: "100%",
        overflow: "hidden",
        WebkitFontSmoothing: "antialiased",
    },
    mark: {
        fill: "none",
        flexShrink: 0,
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 2,
    },

    // server rail
    rail: {
        alignItems: "center",
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "8px",
        paddingBlock: "12px",
    },
    slot: {
        display: "flex",
        justifyContent: "center",
        position: "relative",
        width: "100%",
    },
    server: {
        alignItems: "center",
        backgroundColor: discord.chat,
        borderRadius: "50%",
        color: discord.header,
        display: "inline-flex",
        fontSize: "15px",
        fontWeight: 500,
        height: "48px",
        justifyContent: "center",
        width: "48px",
    },
    home: {
        backgroundColor: discord.blurple,
        borderRadius: "16px",
    },
    logo: {
        fill: discord.white,
        height: "28px",
        width: "28px",
    },
    chosen: {
        backgroundColor: site.space,
        borderRadius: "16px",
        color: discord.white,
    },
    pill: {
        backgroundColor: discord.white,
        borderEndEndRadius: "4px",
        borderStartEndRadius: "4px",
        height: "40px",
        insetInlineStart: 0,
        position: "absolute",
        top: "4px",
        width: "4px",
    },
    unread: {
        backgroundColor: discord.white,
        borderEndEndRadius: "4px",
        borderStartEndRadius: "4px",
        height: "8px",
        insetInlineStart: 0,
        position: "absolute",
        top: "20px",
        width: "4px",
    },
    separator: {
        backgroundColor: discord.divider,
        borderRadius: "1px",
        height: "2px",
        width: "32px",
    },
    add: {
        color: discord.online,
    },

    // channels
    sidebar: {
        backgroundColor: discord.sidebar,
        borderStartStartRadius: "8px",
        display: { default: "grid", [site.narrow]: "none" },
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minHeight: 0,
    },
    serverName: {
        alignItems: "center",
        borderBottomColor: discord.rail,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: discord.header,
        display: "flex",
        fontSize: "16px",
        fontWeight: 600,
        height: "48px",
        justifyContent: "space-between",
        paddingInline: "16px",
    },
    channels: {
        display: "flex",
        flexDirection: "column",
        gap: "2px",
        overflow: "hidden",
        paddingBlockStart: "8px",
        paddingInline: "8px",
    },
    category: {
        alignItems: "center",
        color: discord.muted,
        display: "flex",
        fontSize: "12px",
        fontWeight: 600,
        gap: "2px",
        letterSpacing: "0.02em",
        paddingBlockEnd: "4px",
        paddingBlockStart: "16px",
        textTransform: "uppercase",
    },
    channel: {
        alignItems: "center",
        borderRadius: "4px",
        color: discord.channel,
        display: "flex",
        fontSize: "16px",
        fontWeight: 500,
        gap: "6px",
        height: "32px",
        paddingInline: "8px",
        position: "relative",
    },
    channelUnread: {
        color: discord.header,
        fontWeight: 600,
    },
    channelOn: {
        backgroundColor: discord.selected,
        color: discord.white,
    },
    unreadMark: {
        backgroundColor: discord.white,
        borderEndEndRadius: "4px",
        borderStartEndRadius: "4px",
        height: "8px",
        insetInlineStart: "-8px",
        position: "absolute",
        width: "4px",
    },
    voice: {
        alignItems: "center",
        color: discord.muted,
        display: "flex",
        fontSize: "14px",
        gap: "8px",
        paddingBlock: "3px",
        paddingInlineStart: "36px",
    },
    voiceFace: {
        alignItems: "center",
        borderRadius: "50%",
        color: discord.white,
        display: "inline-flex",
        fontSize: "11px",
        height: "24px",
        justifyContent: "center",
        width: "24px",
    },
    faceMaya: {
        backgroundColor: discord.maya,
    },
    user: {
        alignItems: "center",
        backgroundColor: discord.panel,
        color: discord.muted,
        display: "flex",
        gap: "8px",
        height: "52px",
        paddingInline: "8px",
    },
    userFace: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: discord.white,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "13px",
        height: "32px",
        justifyContent: "center",
        position: "relative",
        width: "32px",
    },
    status: {
        backgroundColor: discord.online,
        borderColor: discord.panel,
        borderRadius: "50%",
        borderStyle: "solid",
        borderWidth: "3px",
        bottom: "-3px",
        height: "10px",
        insetInlineEnd: "-3px",
        position: "absolute",
        width: "10px",
    },
    userName: {
        display: "grid",
        flexGrow: 1,
        fontSize: "12px",
        lineHeight: 1.3,
        minWidth: 0,
    },

    // the channel
    main: {
        backgroundColor: discord.chat,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        borderBottomColor: discord.rail,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: discord.channel,
        display: "flex",
        gap: "8px",
        height: "48px",
        overflow: "hidden",
        paddingInline: "16px",
    },
    headName: {
        color: discord.header,
        fontSize: "16px",
        fontWeight: 600,
    },
    topic: {
        borderInlineStartColor: discord.divider,
        borderInlineStartStyle: "solid",
        borderInlineStartWidth: "1px",
        color: discord.muted,
        fontSize: "14px",
        marginInlineStart: "4px",
        overflow: "hidden",
        paddingInlineStart: "12px",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    headTools: {
        alignItems: "center",
        display: { default: "flex", [site.narrow]: "none" },
        flexShrink: 0,
        gap: "16px",
        marginInlineStart: "auto",
    },
    search: {
        alignItems: "center",
        backgroundColor: discord.rail,
        borderRadius: "4px",
        color: discord.muted,
        display: "flex",
        fontSize: "14px",
        gap: "40px",
        height: "24px",
        paddingInline: "8px",
    },
    messages: {
        display: "flex",
        flexDirection: "column",
        justifyContent: "flex-end",
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        padding: 0,
        paddingBlockEnd: "16px",
    },
    divider: {
        alignItems: "center",
        color: discord.muted,
        display: "flex",
        fontSize: "12px",
        fontWeight: 600,
        gap: "4px",
        marginBlockEnd: "8px",
        marginBlockStart: "16px",
        marginInline: "16px",
        "::after": { backgroundColor: discord.divider, content: '""', flexGrow: 1, height: "1px" },
        "::before": { backgroundColor: discord.divider, content: '""', flexGrow: 1, height: "1px" },
    },
    message: {
        display: "grid",
        gap: "2px",
        lineHeight: 1.375,
        marginBlockStart: "17px",
        minHeight: "44px",
        paddingInlineEnd: "48px",
        paddingInlineStart: "72px",
        position: "relative",
    },
    avatar: {
        alignItems: "center",
        borderRadius: "50%",
        color: discord.white,
        display: "inline-flex",
        fontSize: "16px",
        height: "40px",
        insetInlineStart: "16px",
        justifyContent: "center",
        position: "absolute",
        top: "2px",
        width: "40px",
    },
    faceAgent: {
        backgroundColor: site.agent,
    },
    faceFlorian: {
        backgroundColor: site.avatar,
    },
    byline: {
        alignItems: "center",
        display: "flex",
        gap: "4px",
    },
    nameOwner: {
        color: discord.owner,
        fontWeight: 500,
    },
    nameAgent: {
        color: discord.header,
        fontWeight: 500,
    },
    app: {
        alignItems: "center",
        backgroundColor: discord.blurple,
        borderRadius: "4px",
        color: discord.white,
        display: "inline-flex",
        fontSize: "10px",
        fontWeight: 600,
        gap: "2px",
        height: "15px",
        paddingInline: "4px",
    },
    time: {
        color: discord.muted,
        fontSize: "12px",
        marginInlineStart: "4px",
    },
    text: {
        color: discord.text,
    },
    reactions: {
        display: "flex",
        gap: "4px",
        marginBlockStart: "2px",
    },
    reaction: {
        alignItems: "center",
        backgroundColor: discord.sidebar,
        borderColor: "transparent",
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: discord.text,
        display: "inline-flex",
        fontSize: "14px",
        gap: "6px",
        paddingBlock: "2px",
        paddingInline: "6px",
    },
    reactionMine: {
        backgroundColor: discord.reacted,
        borderColor: discord.blurple,
    },
    thread: {
        backgroundColor: discord.sidebar,
        borderRadius: "4px",
        display: "grid",
        fontSize: "14px",
        gap: "2px",
        gridTemplateColumns: "auto auto",
        justifyContent: "start",
        columnGap: "8px",
        marginBlockStart: "4px",
        maxWidth: "400px",
        paddingBlock: "8px",
        paddingInline: "12px",
    },
    threadName: {
        color: discord.header,
        fontWeight: 600,
    },
    threadCount: {
        color: discord.link,
        fontWeight: 600,
    },
    threadLast: {
        color: discord.muted,
        gridColumn: "1 / -1",
    },
    composer: {
        alignItems: "center",
        backgroundColor: discord.field,
        borderRadius: "8px",
        color: discord.muted,
        display: "flex",
        gap: "16px",
        marginBlockEnd: "24px",
        marginInline: "16px",
        minHeight: "44px",
        paddingInline: "16px",
    },
    placeholder: {
        flexGrow: 1,
        fontSize: "16px",
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    gif: {
        borderColor: "currentColor",
        borderRadius: "4px",
        borderStyle: "solid",
        borderWidth: "2px",
        fontSize: "10px",
        fontWeight: 800,
        paddingInline: "3px",
    },
});
