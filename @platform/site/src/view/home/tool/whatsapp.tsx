import * as style from "@destack/style";

import { messages } from "../verb/corner";
import { site } from "../figure/site.stylex";
import { whatsapp } from "./whatsapp.stylex";

/** The strokes of each of the app's own glyphs, on a 24-unit grid. */
const glyphs = {
    chats: ["M5 18.5 6 15a7.5 7.5 0 1 1 3 3z"],
    status: ["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10z"],
    channels: ["M4 9h3l7-5v16l-7-5H4z", "M18 9a4 4 0 0 1 0 6"],
    communities: [
        "M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M16 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6z",
        "M3 19a5 5 0 0 1 10 0",
        "M11 19a5 5 0 0 1 10 0",
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
    compose: ["M4 20h4L19 9l-4-4L4 16z", "m13.5 6.5 4 4"],
    menu: ["M12 5h.01", "M12 12h.01", "M12 19h.01"],
    search: ["M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13z", "m20 20-4.8-4.8"],
    video: ["m16 10 5-3v10l-5-3", "M3 6h13v12H3z"],
    plus: ["M12 5v14", "M5 12h14"],
    smile: [
        "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z",
        "M8.5 14.5a4.5 4.5 0 0 0 7 0",
        "M9 10h.01",
        "M15 10h.01",
    ],
    mic: [
        "M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3z",
        "M19 11a7 7 0 0 1-14 0",
        "M12 18v3",
    ],
} satisfies Record<string, readonly string[]>;

/** The rail's places, Chats chosen. */
const places: readonly (keyof typeof glyphs)[] = ["chats", "status", "channels", "communities"];

/** The chats the list shows: name, colour, last line, time, and unread count. */
const chats: readonly (readonly [
    name: string,
    tint: string,
    line: string,
    time: string,
    unread: number,
])[] = [
    ["Destack launch", "#c64a17", "You: The first one. We have a reputation…", "10:02", 0],
    ["Ines Ferreira", "#a0485f", "Wednesday at 11 works 👍", "09:58", 2],
    ["Design", "#3f7d88", "Jonas: New mockups are up", "09:31", 0],
    ["Tara Okafor", "#4f8a5b", "Do invites go out by signup date?", "Yesterday", 0],
];

/** The filters over the chat list, All chosen. */
const filters: readonly string[] = ["All", "Unread", "Favourites", "Groups"];

/** Draw the launch conversation as a group chat in the browser, your messages on the right. */
export function WhatsApp() {
    return (
        <div data-component="WhatsAppPage" {...style.attrs(styles.page)}>
            {/* navigation rail */}
            <nav {...style.attrs(styles.rail)}>
                {places.map((place) => (
                    <span {...style.attrs(styles.place, place === "chats" && styles.placeOn)}>
                        <Mark name={place} size={22} />
                    </span>
                ))}
                <span {...style.attrs(styles.railEnd)}>
                    <Mark name="gear" size={22} />
                    <span {...style.attrs(styles.me)}>F</span>
                </span>
            </nav>

            {/* chat list */}
            <nav {...style.attrs(styles.list)}>
                <span {...style.attrs(styles.listHead)}>
                    <b {...style.attrs(styles.listTitle)}>Chats</b>
                    <Mark name="compose" size={20} />
                    <Mark name="menu" size={20} />
                </span>
                <span {...style.attrs(styles.search)}>
                    <Mark name="search" size={16} />
                    Search or start a new chat
                </span>
                <span {...style.attrs(styles.filters)}>
                    {filters.map((filter) => (
                        <span {...style.attrs(styles.filter, filter === "All" && styles.filterOn)}>
                            {filter}
                        </span>
                    ))}
                </span>
                {chats.map(([name, tint, line, time, unread], index) => (
                    <span {...style.attrs(styles.chat, index === 0 && styles.chatOn)}>
                        <span
                            style={{ "background-color": tint }}
                            {...style.attrs(styles.chatFace)}
                        >
                            {name.slice(0, 1)}
                        </span>
                        <span {...style.attrs(styles.chatText)}>
                            <span {...style.attrs(styles.chatTop)}>
                                <span {...style.attrs(styles.chatName)}>{name}</span>
                                <span
                                    {...style.attrs(
                                        styles.chatTime,
                                        unread > 0 && styles.chatTimeUnread,
                                    )}
                                >
                                    {time}
                                </span>
                            </span>
                            <span {...style.attrs(styles.chatTop)}>
                                <span {...style.attrs(styles.chatLine)}>
                                    {index === 0 ? (
                                        <span {...style.attrs(styles.ticks)}>✓✓ </span>
                                    ) : undefined}
                                    {line}
                                </span>
                                {unread > 0 ? (
                                    <span {...style.attrs(styles.badge)}>{unread}</span>
                                ) : undefined}
                            </span>
                        </span>
                    </span>
                ))}
            </nav>

            {/* the conversation */}
            <main {...style.attrs(styles.main)}>
                <header {...style.attrs(styles.head)}>
                    <span {...style.attrs(styles.chatFace, styles.headFace, styles.groupFace)}>
                        D
                    </span>
                    <span {...style.attrs(styles.headText)}>
                        <span {...style.attrs(styles.headName)}>Destack launch</span>
                        <span>Agent, Jonas, Maya, You</span>
                    </span>
                    <Mark name="video" size={22} />
                    <Mark name="search" size={20} />
                    <Mark name="menu" size={20} />
                </header>

                <ol {...style.attrs(styles.messages)}>
                    <li {...style.attrs(styles.day)}>Today</li>
                    {messages.map(([author, time, text], index) => (
                        <li
                            {...style.attrs(
                                styles.bubble,
                                author === "Florian" ? styles.outgoing : styles.incoming,
                                author === "Agent" &&
                                    messages[index - 1]?.[0] === "Agent" &&
                                    styles.continued,
                            )}
                        >
                            {author === "Agent" && messages[index - 1]?.[0] !== "Agent" ? (
                                <b {...style.attrs(styles.sender)}>Agent</b>
                            ) : undefined}
                            <span>
                                {text}
                                <span {...style.attrs(styles.spacer)} />
                            </span>
                            <span {...style.attrs(styles.time)}>
                                {time}
                                {author === "Florian" ? (
                                    <span {...style.attrs(styles.ticks)}>✓✓</span>
                                ) : undefined}
                            </span>
                        </li>
                    ))}
                </ol>

                <span {...style.attrs(styles.composer)}>
                    <Mark name="plus" size={24} />
                    <span {...style.attrs(styles.field)}>
                        <Mark name="smile" size={22} />
                        Type a message
                    </span>
                    <Mark name="mic" size={22} />
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

/** The WhatsApp page styles. */
const styles = style.create({
    page: {
        backgroundColor: whatsapp.list,
        color: whatsapp.ink,
        display: "grid",
        fontFamily: whatsapp.face,
        fontSize: "14.2px",
        gridTemplateColumns: {
            default: "64px 300px minmax(0, 1fr)",
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
        strokeWidth: 1.7,
    },

    // navigation rail
    rail: {
        alignItems: "center",
        backgroundColor: whatsapp.bar,
        borderInlineEndColor: whatsapp.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: whatsapp.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        gap: "8px",
        paddingBlock: "12px",
    },
    place: {
        alignItems: "center",
        borderRadius: "50%",
        display: "inline-flex",
        height: "40px",
        justifyContent: "center",
        width: "40px",
    },
    placeOn: {
        backgroundColor: whatsapp.line,
        color: whatsapp.ink,
    },
    railEnd: {
        alignItems: "center",
        display: "flex",
        flexDirection: "column",
        gap: "14px",
        marginBlockStart: "auto",
    },
    me: {
        alignItems: "center",
        backgroundColor: site.avatar,
        borderRadius: "50%",
        color: whatsapp.onAccent,
        display: "inline-flex",
        fontSize: "13px",
        height: "32px",
        justifyContent: "center",
        width: "32px",
    },

    // chat list
    list: {
        borderInlineEndColor: whatsapp.line,
        borderInlineEndStyle: "solid",
        borderInlineEndWidth: "1px",
        color: whatsapp.muted,
        display: { default: "flex", [site.narrow]: "none" },
        flexDirection: "column",
        overflow: "hidden",
    },
    listHead: {
        alignItems: "center",
        display: "flex",
        gap: "20px",
        height: "60px",
        paddingInline: "16px",
    },
    listTitle: {
        color: whatsapp.ink,
        flexGrow: 1,
        fontSize: "22px",
    },
    search: {
        alignItems: "center",
        backgroundColor: whatsapp.bar,
        borderRadius: "8px",
        display: "flex",
        gap: "16px",
        height: "35px",
        marginInline: "12px",
        paddingInline: "12px",
        whiteSpace: "nowrap",
    },
    filters: {
        display: "flex",
        gap: "8px",
        paddingBlock: "10px",
        paddingInline: "12px",
    },
    filter: {
        backgroundColor: whatsapp.bar,
        borderRadius: "999px",
        fontSize: "14px",
        paddingBlock: "4px",
        paddingInline: "12px",
    },
    filterOn: {
        backgroundColor: whatsapp.filter,
        color: whatsapp.filterInk,
    },
    chat: {
        alignItems: "center",
        display: "flex",
        gap: "12px",
        paddingInline: "12px",
    },
    chatOn: {
        backgroundColor: whatsapp.selected,
    },
    chatFace: {
        alignItems: "center",
        borderRadius: "50%",
        color: whatsapp.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "18px",
        height: "49px",
        justifyContent: "center",
        width: "49px",
    },
    chatText: {
        borderBottomColor: whatsapp.line,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        display: "grid",
        flexGrow: 1,
        gap: "2px",
        gridTemplateColumns: "minmax(0, 1fr)",
        minWidth: 0,
        paddingBlock: "13px",
    },
    chatTop: {
        alignItems: "center",
        display: "flex",
        gap: "6px",
        justifyContent: "space-between",
    },
    chatName: {
        color: whatsapp.ink,
        fontSize: "16px",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    chatTime: {
        flexShrink: 0,
        fontSize: "12px",
    },
    chatTimeUnread: {
        color: whatsapp.green,
    },
    chatLine: {
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    badge: {
        alignItems: "center",
        backgroundColor: whatsapp.green,
        borderRadius: "999px",
        color: whatsapp.onAccent,
        display: "inline-flex",
        flexShrink: 0,
        fontSize: "12px",
        fontWeight: 600,
        height: "20px",
        justifyContent: "center",
        minWidth: "20px",
    },

    // the conversation
    main: {
        backgroundColor: whatsapp.wallpaper,
        backgroundImage: whatsapp.doodles,
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr) auto",
        minWidth: 0,
    },
    head: {
        alignItems: "center",
        backgroundColor: whatsapp.bar,
        color: whatsapp.muted,
        display: "flex",
        gap: "16px",
        height: "60px",
        paddingInline: "16px",
    },
    headFace: {
        fontSize: "16px",
        height: "40px",
        width: "40px",
    },
    groupFace: {
        backgroundColor: site.space,
    },
    headText: {
        display: "grid",
        flexGrow: 1,
        fontSize: "13px",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    headName: {
        color: whatsapp.ink,
        fontSize: "16px",
    },
    messages: {
        display: "flex",
        flexDirection: "column",
        gap: "4px",
        justifyContent: "flex-end",
        listStyle: "none",
        margin: 0,
        overflow: "hidden",
        paddingBlock: "12px",
        paddingInline: { default: "6%", [site.narrow]: "14px" },
    },
    day: {
        alignSelf: "center",
        backgroundColor: whatsapp.incoming,
        borderRadius: "8px",
        boxShadow: whatsapp.lift,
        color: whatsapp.muted,
        fontSize: "12.5px",
        marginBlockEnd: "8px",
        paddingBlock: "5px",
        paddingInline: "12px",
    },
    bubble: {
        borderRadius: "7.5px",
        boxShadow: whatsapp.lift,
        display: "grid",
        flexShrink: 0,
        lineHeight: 1.35,
        marginBlockStart: "8px",
        maxWidth: "65%",
        paddingBlockEnd: "8px",
        paddingBlockStart: "6px",
        paddingInlineEnd: "7px",
        paddingInlineStart: "9px",
        position: "relative",
    },
    outgoing: {
        alignSelf: "flex-end",
        backgroundColor: whatsapp.outgoing,
        borderStartEndRadius: 0,
        "::before": {
            backgroundColor: whatsapp.outgoing,
            clipPath: "polygon(0 0, 100% 0, 0 100%)",
            content: '""',
            height: "13px",
            insetInlineEnd: "-8px",
            position: "absolute",
            top: 0,
            width: "8px",
        },
    },
    incoming: {
        alignSelf: "flex-start",
        backgroundColor: whatsapp.incoming,
        borderStartStartRadius: 0,
        "::before": {
            backgroundColor: whatsapp.incoming,
            clipPath: "polygon(100% 0, 0 0, 100% 100%)",
            content: '""',
            height: "13px",
            insetInlineStart: "-8px",
            position: "absolute",
            top: 0,
            width: "8px",
        },
    },
    continued: {
        borderStartStartRadius: "7.5px",
        marginBlockStart: "2px",
        "::before": { content: "none" },
    },
    sender: {
        color: site.agent,
        fontSize: "12.8px",
        fontWeight: 500,
        paddingBlockEnd: "2px",
    },
    spacer: {
        display: "inline-block",
        width: "58px",
    },
    time: {
        bottom: "4px",
        color: whatsapp.muted,
        display: "flex",
        fontSize: "11px",
        gap: "3px",
        insetInlineEnd: "7px",
        position: "absolute",
    },
    ticks: {
        color: whatsapp.tick,
        letterSpacing: "-0.25em",
    },
    composer: {
        alignItems: "center",
        backgroundColor: whatsapp.bar,
        color: whatsapp.muted,
        display: "flex",
        gap: "16px",
        minHeight: "62px",
        paddingInline: "16px",
    },
    field: {
        alignItems: "center",
        backgroundColor: whatsapp.list,
        borderRadius: "8px",
        display: "flex",
        flexGrow: 1,
        gap: "12px",
        height: "42px",
        paddingInline: "12px",
    },
});
